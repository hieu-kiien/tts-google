<script lang="ts">
  import { projectState } from "../state/projectState.svelte";
  import { uiState } from "../state/uiState.svelte";
  import { playerState } from "../state/playerState.svelte";
  import type { SegmentRecord, TextChunk } from "../types/tts";
  import { 
    getProjectSegments, 
    splitSegment, 
    mergeSegments, 
    deleteSegment, 
    deleteSegmentsBatch, 
    moveSegment, 
    insertSegmentAt,
    rechunkProjectSegments,
    chunkTextPreview
  } from "../api/projectClient";
  import { synthesizePreviewAudio } from "../api/audioClient";
  import { toastStore } from "../state/toasts.svelte";
  import { enqueueProject, pauseProject, resumeProject } from "../api/queueClient";
  import { getErrorMessage } from "../utils/errorUtils";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import SegmentList from "./SegmentList.svelte";
  import SmartErrorBanner from "./SmartErrorBanner.svelte";
  import BatchActionsBar from "./BatchActionsBar.svelte";

  let searchQuery = $state("");
  let showReplace = $state(false);
  let replaceText = $state("");

  async function handleReplaceAll() {
    if (!searchQuery.trim()) {
      toastStore.showInfo("Vui lòng nhập từ/cụm từ cần tìm kiếm vào ô tìm kiếm.");
      return;
    }
    const query = searchQuery;
    const replacement = replaceText;

    try {
      const count = await projectState.replaceAllText(query, replacement);
      if (count > 0) {
        toastStore.showSuccess(`Đã thay thế ${count} cụm từ "${query}" ➔ "${replacement}" và tự động lưu vào database.`);
      } else {
        toastStore.showInfo(`Không tìm thấy cụm từ "${query}" trong dự án.`);
      }
    } catch (err: unknown) {
      toastStore.showError(`Lỗi thay thế: ${getErrorMessage(err)}`);
    }
  }

  let currentSource = $derived(projectState.currentProject?.source_text || "");

  // References to textarea elements for cursor extraction in segment splitting (R5)
  let textareaElements: Record<string, HTMLTextAreaElement | null> = $state({});
  let isSynthesizingPreview = $state(false);

  // Segment list derived from projectState or generated from source text
  let segments = $derived.by<SegmentRecord[]>(() => {
    // If we have real segments from DB, use them
    if (projectState.segments.length > 0) return projectState.segments;
    // Only generate fallback segments for new projects that haven't been saved yet
    // (they have source_text but no segments in DB)
    if (!projectState.currentProject?.id) {
      const lines = currentSource.split("\n").filter((l: string) => l.trim().length > 0);
      return lines.map((line: string, idx: number): SegmentRecord => ({
        id: `seg_${idx + 1}`,
        project_id: projectState.currentProject?.id || "1",
        position: idx + 1,
        text: line,
        spoken_text: projectState.computeSpokenText(line),
        prompt: "Đọc tự nhiên, truyền cảm",
        status: "pending",
        is_locked: false,
        is_skipped: false,
        attempts: 0,
        duration_ms: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        attempt_count: 0,
        cancel_requested: false,
        state_revision: 1,
        output_size: 0
      }));
    }
    // Project exists in DB but segments not loaded yet — return empty, don't generate ghosts
    return [];
  });

  // Derived completed segment count & guided workflow stepper step (R6)
  let completedCount = $derived.by(() => {
    return segments.filter(s => s.status === 'success' || s.status === 'approved').length;
  });

  let currentStep = $derived.by(() => {
    if (uiState.activeView === 'export') return 4;
    if (completedCount > 0 && completedCount === segments.length) return 4;
    if (completedCount > 0) return 3;
    if (projectState.currentProject?.voice) return 2;
    if (segments.length > 0) return 1;
    return 1;
  });

  // Smart Error Recovery Banner state detection (R3)
  let activeErrorSegment = $derived.by(() => {
    return segments.find(s => s.status === 'failed' || s.status === 'retry_wait' || !!s.error_message || !!s.last_error_message);
  });

  function handleSegmentClick(seg: SegmentRecord) {
    projectState.activeSegmentId = seg.id;
  }

  // Handle segment text change & stale state transition (R1)
  function handleSegmentTextInput(seg: SegmentRecord) {
    if (seg.status === 'success' || seg.status === 'approved') {
      seg.status = 'stale';
    }
    seg.spoken_text = projectState.computeSpokenText(seg.text);
    projectState.updateSegmentText(seg.id, seg.text);
  }

  // Re-synthesize single segment (for R1 stale audio & R3 smart error recovery)
  async function handleResynthesizeSegment(seg: SegmentRecord) {
    const projectId = projectState.currentProject?.id || seg.project_id;
    if (!projectId) {
      toastStore.showError("Không tìm thấy dự án.");
      return;
    }

    seg.audio_path = undefined;
    seg.status = 'processing';
    seg.error_message = undefined;
    seg.error_code = undefined;
    seg.last_error_message = undefined;

    toastStore.showInfo(`Đang đưa đoạn #${seg.position} vào hàng đợi thử lại...`);
    try {
      await invoke('requeue_segment', { projectId, segmentId: seg.id });
      // Update UI state to queued instead of waiting for sync
      seg.status = 'queued';
      toastStore.showSuccess(`Đã đưa đoạn #${seg.position} vào hàng đợi!`);
    } catch (err: unknown) {
      seg.status = 'failed';
      seg.error_message = getErrorMessage(err);
      toastStore.showError(`Lỗi khi đưa đoạn #${seg.position} vào hàng đợi: ${getErrorMessage(err)}`);
    }
  }

  const previewAudioCache = new Map<string, { data_url: string; duration_ms: number }>();
  let isRechunking = $state(false);

  async function handleRechunkAllText() {
    if (!confirm(
      "⚠️ CẢNH BÁO — HÀNH ĐỘNG KHÔNG THỂ HOÀN TÁC\n\n" +
      "Chia lại văn bản sẽ:\n" +
      "• XÓA toàn bộ file audio đã tạo\n" +
      "• XÓA mọi chỉnh sửa thủ công trên từng đoạn\n" +
      "• Tạo lại segments mới — cần TỐN THÊM quota API để tổng hợp lại\n\n" +
      "Bạn có chắc chắn muốn tiếp tục?"
    )) {
      return;
    }
    const proj = projectState.currentProject;
    let textToChunk = proj?.source_text;
    
    if (!textToChunk || !textToChunk.trim()) {
      textToChunk = segments.map((s: SegmentRecord) => s.text).join("\n\n");
    }

    if (!textToChunk || !textToChunk.trim()) {
      toastStore.showError("Chưa có văn bản nào để tự động phân đoạn.");
      return;
    }

    try {
      isRechunking = true;
      toastStore.showInfo("Đang tự động chia nhỏ văn bản theo tiêu đề, đoạn văn và câu...");
      
      if (proj?.id) {
        const updatedSegs = await rechunkProjectSegments(proj.id, textToChunk, "auto");
        projectState.segments = updatedSegs;
        toastStore.showSuccess(`Đã tự động chia nhỏ thành ${updatedSegs.length} đoạn audio (30-60s)!`);
      } else {
        const chunks = await chunkTextPreview(textToChunk, "auto");
        const newSegs: SegmentRecord[] = chunks.map((c: TextChunk, idx: number) => ({
          id: `seg_${idx + 1}`,
          project_id: "temp",
          position: idx + 1,
          text: c.text,
          spoken_text: projectState.computeSpokenText(c.text),
          prompt: "Đọc tự nhiên",
          status: "pending",
          is_locked: false,
          is_skipped: false,
          attempts: 0,
          duration_ms: c.estimated_duration_ms || 0,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
          attempt_count: 0,
          cancel_requested: false,
          state_revision: 1,
          output_size: 0
        }));
        projectState.segments = newSegs;
        toastStore.showSuccess(`Đã tự động chia nhỏ thành ${newSegs.length} đoạn audio!`);
      }
    } catch (err: unknown) {
      toastStore.showError(`Lỗi phân đoạn: ${getErrorMessage(err)}`);
    } finally {
      isRechunking = false;
    }
  }

  async function handlePlayPreview(seg: SegmentRecord) {
    if (seg.audio_path && seg.status !== 'stale') {
      playerState.playUrl(seg.audio_path, seg.id);
      return;
    }

    const textToSynthesize = (seg?.spoken_text || seg?.text || currentSource).trim();
    if (!textToSynthesize) {
      toastStore.showError("Vui lòng nhập văn bản trước khi nghe thử!");
      return;
    }

    const voice = seg?.voice || projectState.currentProject?.voice || "Kore";
    const cacheKey = `${voice}_${textToSynthesize}`;

    if (previewAudioCache.has(cacheKey)) {
      const cached = previewAudioCache.get(cacheKey)!;
      if (seg) seg.duration_ms = cached.duration_ms;
      playerState.playUrl(cached.data_url, seg?.id || "preview_temp");
      toastStore.showSuccess("Phát nhanh audio từ bộ nhớ đệm ⚡");
      return;
    }

    try {
      isSynthesizingPreview = true;
      toastStore.showInfo(`Đang tạo giọng đọc (${voice}) cho đoạn #${seg?.position || 1}...`);
      
      const res = await synthesizePreviewAudio({
        text: textToSynthesize,
        voice,
        model: projectState.currentProject?.model || "gemini-3.1-flash-tts-preview",
        speed: 1.0,
        pitch: 1.0
      });

      // LRU logic to prevent memory bloat
      if (previewAudioCache.size >= 10) {
        const firstKey = previewAudioCache.keys().next().value;
        if (firstKey) previewAudioCache.delete(firstKey);
      }
      previewAudioCache.set(cacheKey, { data_url: res.data_url, duration_ms: res.duration_ms });
      if (seg) seg.duration_ms = res.duration_ms;

      playerState.playUrl(res.data_url, seg?.id || "preview_temp");
      toastStore.showSuccess(`Tạo audio thành công (${(res.duration_ms / 1000).toFixed(1)}s)!`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi tạo audio: ${getErrorMessage(err)}`);
    } finally {
      isSynthesizingPreview = false;
    }
  }

  // Split segment handler with SQLite DB IPC persistence (R5)
  async function handleSplitSegment(seg: SegmentRecord) {
    const el = textareaElements[seg.id] || (document.querySelector(`#textarea-${seg.id}`) as HTMLTextAreaElement);
    
    if (document.activeElement !== el) {
      toastStore.showWarning("Vui lòng click chuột vào vị trí muốn tách trong khung soạn thảo.");
      return;
    }

    let splitIndex = el ? el.selectionStart : -1;
    
    if (splitIndex <= 0 || splitIndex >= seg.text.length) {
      toastStore.showError("Vị trí con trỏ không hợp lệ. Vui lòng đặt con trỏ ở giữa đoạn văn bản.");
      return;
    }

    const projectId = projectState.currentProject?.id;
    if (!projectId) {
      toastStore.showError("Chưa có dự án nào được chọn để tách đoạn.");
      return;
    }

    try {
      // Flush pending text edits before splitting to avoid data loss
      await projectState.flushPendingSaves();
      toastStore.showInfo(`Đang tách đoạn #${seg.position} tại vị trí ký tự ${splitIndex}...`);
      await splitSegment(projectId, seg.id, splitIndex);
      
      const updatedSegs = await getProjectSegments(projectId);
      projectState.segments = updatedSegs;
      toastStore.showSuccess(`Đã tách đoạn #${seg.position} thành 2 đoạn riêng biệt thành công!`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi tách đoạn: ${getErrorMessage(err)}`);
    }
  }

  // Trigger batch audio generation queue for Step 3 in Stepper (R6)
  async function handleTriggerQueue() {
    const projectId = projectState.currentProject?.id;
    if (!projectId) {
      toastStore.showError("Chưa có dự án nào được chọn.");
      return;
    }
    try {
      toastStore.showInfo("Đang đưa các đoạn văn bản vào hàng đợi xử lý âm thanh...");
      await enqueueProject(projectId);
      const updated = await getProjectSegments(projectId);
      projectState.segments = updated;
      toastStore.showSuccess("Đã khởi động hàng đợi tạo âm thanh thành công!");
    } catch (err: unknown) {
      toastStore.showError("Lỗi khởi động hàng đợi: " + getErrorMessage(err));
    }
  }

  // Merge segment with previous (UX Fix)
  async function handleMergeWithPrevious(seg: SegmentRecord) {
    if (seg.position <= 1) {
      toastStore.showError("Đoạn đầu tiên không thể gộp với đoạn trước.");
      return;
    }
    const projectId = projectState.currentProject?.id;
    if (!projectId) {
      toastStore.showError("Chưa có dự án nào được chọn.");
      return;
    }
    try {
      // Flush pending text edits before merging to avoid data loss
      await projectState.flushPendingSaves();
      toastStore.showInfo(`Đang gộp đoạn #${seg.position} với đoạn #${seg.position - 1}...`);
      await mergeSegments(projectId, seg.id);
      const updatedSegs = await getProjectSegments(projectId);
      projectState.segments = updatedSegs;
      toastStore.showSuccess(`Đã gộp thành công thành đoạn #${seg.position - 1}!`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi gộp đoạn: ${getErrorMessage(err)}`);
    }
  }

  // Segment multi-selection state for batch actions
  let selectedSegmentIds = $state<Set<string>>(new Set());

  const isAllSegmentsSelected = $derived.by(() => {
    if (segments.length === 0) return false;
    return segments.every((s: SegmentRecord) => selectedSegmentIds.has(s.id));
  });

  function toggleSelectAllSegments() {
    if (isAllSegmentsSelected) {
      selectedSegmentIds.clear();
    } else {
      selectedSegmentIds = new Set(segments.map((s: SegmentRecord) => s.id));
    }
  }

  function toggleSelectSegment(id: string, e: Event) {
    e.stopPropagation();
    const next = new Set(selectedSegmentIds);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    selectedSegmentIds = next;
  }

  async function handleDeleteSingleSegment(seg: SegmentRecord) {
    if (!confirm(`Bạn có chắc chắn muốn xóa đoạn #${seg.position}?`)) return;
    
    if (playerState.currentPlayingSegmentId === seg.id) {
      playerState.stop();
    }
    if (projectState.activeSegmentId === seg.id) {
      projectState.activeSegmentId = null;
    }

    const projectId = projectState.currentProject?.id;

    if (!projectId) {
      projectState.segments = projectState.segments
        .filter((s: SegmentRecord) => s.id !== seg.id)
        .map((s: SegmentRecord, idx: number) => ({ ...s, position: idx + 1 }));
      toastStore.showSuccess(`Đã xóa đoạn #${seg.position}.`);
      return;
    }

    try {
      // Flush pending text edits before deleting to avoid data loss
      await projectState.flushPendingSaves();
      toastStore.showInfo(`Đang xóa đoạn #${seg.position}...`);
      await deleteSegment(projectId, seg.id);
      const updated = await getProjectSegments(projectId);
      projectState.segments = updated;
      const nextSel = new Set(selectedSegmentIds);
      nextSel.delete(seg.id);
      selectedSegmentIds = nextSel;
      toastStore.showSuccess(`Đã xóa đoạn #${seg.position} thành công!`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi xóa đoạn: ${getErrorMessage(err)}`);
    }
  }

  async function handleDeleteSelectedSegments() {
    if (selectedSegmentIds.size === 0) return;
    const ids = Array.from(selectedSegmentIds);
    if (!confirm(`Bạn có chắc chắn muốn xóa ${ids.length} đoạn đã chọn?`)) return;

    if (playerState.currentPlayingSegmentId && selectedSegmentIds.has(playerState.currentPlayingSegmentId)) {
      playerState.stop();
    }
    if (projectState.activeSegmentId && selectedSegmentIds.has(projectState.activeSegmentId)) {
      projectState.activeSegmentId = null;
    }

    const projectId = projectState.currentProject?.id;
    if (!projectId) {
      projectState.segments = projectState.segments
        .filter((s: SegmentRecord) => !selectedSegmentIds.has(s.id))
        .map((s: SegmentRecord, idx: number) => ({ ...s, position: idx + 1 }));
      selectedSegmentIds.clear();
      toastStore.showSuccess(`Đã xóa ${ids.length} đoạn.`);
      return;
    }

    try {
      // Flush pending text edits before batch deleting to avoid data loss
      await projectState.flushPendingSaves();
      toastStore.showInfo(`Đang xóa ${ids.length} đoạn đã chọn...`);
      await deleteSegmentsBatch(projectId, ids);
      const updated = await getProjectSegments(projectId);
      projectState.segments = updated;
      selectedSegmentIds.clear();
      toastStore.showSuccess(`Đã xóa ${ids.length} đoạn thành công!`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi xóa hàng loạt: ${getErrorMessage(err)}`);
    }
  }

  async function handleMoveSegment(seg: SegmentRecord, direction: "up" | "down") {
    const projectId = projectState.currentProject?.id;
    if (!projectId) {
      toastStore.showInfo("Vui lòng lưu dự án trước khi di chuyển vị trí.");
      return;
    }
    try {
      // Flush pending text edits before moving to avoid stale positions
      await projectState.flushPendingSaves();
      await moveSegment(projectId, seg.id, direction);
      const updated = await getProjectSegments(projectId);
      projectState.segments = updated;
    } catch (err: unknown) {
      toastStore.showError(`Lỗi di chuyển đoạn: ${getErrorMessage(err)}`);
    }
  }

  async function handleInsertSegmentBelow(seg: SegmentRecord) {
    const projectId = projectState.currentProject?.id;
    const targetPos = seg.position + 1;
    if (!projectId) {
      const newSeg: SegmentRecord = {
        id: `seg_${Date.now()}`,
        project_id: "temp",
        position: targetPos,
        text: "Đoạn mới...",
        spoken_text: "Đoạn mới...",
        prompt: "Đọc tự nhiên",
        status: "pending",
        is_locked: false,
        is_skipped: false,
        attempts: 0,
        duration_ms: 0,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        attempt_count: 0,
        cancel_requested: false,
        state_revision: 1,
        output_size: 0
      };
      const list = [...projectState.segments];
      list.splice(seg.position, 0, newSeg);
      projectState.segments = list.map((s, idx) => ({ ...s, position: idx + 1 }));
      toastStore.showSuccess(`Đã thêm đoạn mới ở vị trí #${targetPos}`);
      return;
    }

    try {
      toastStore.showInfo(`Đang thêm đoạn mới phía dưới đoạn #${seg.position}...`);
      await insertSegmentAt(projectId, targetPos, "Đoạn văn bản mới...");
      const updated = await getProjectSegments(projectId);
      projectState.segments = updated;
      toastStore.showSuccess(`Đã thêm đoạn mới ở vị trí #${targetPos}!`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi thêm đoạn: ${getErrorMessage(err)}`);
    }
  }

  // Offline detection (UX Fix)
  let isOffline = $state(typeof navigator !== 'undefined' ? !navigator.onLine : false);

  onMount(() => {
    const handleOnline = () => { isOffline = false; };
    const handleOffline = () => { 
      isOffline = true; 
      if (projectState.currentProject?.id) {
        pauseProject(projectState.currentProject.id).catch(e => console.warn(e));
        toastStore.showInfo("Mất mạng: Đã tự động tạm dừng hàng đợi để tránh lỗi.");
      }
    };
    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  });
</script>

<div class="editor-central-container" role="main" aria-label="Khu vực soạn thảo chính">
  <header class="editor-header">

    <!-- Search & Quick Action Tools -->
    <div class="editor-tools">
      <div class="action-icons-left">
        <button 
          class="icon-btn action-btn-sm" 
          onclick={handleRechunkAllText} 
          disabled={isRechunking}
          title="✂️ Tự động chia nhỏ văn bản dài (Rechunk)"
        >
          {isRechunking ? '⏳' : '✂️'}
        </button>
      </div>
      <div class="search-field">
        <input 
          type="text" 
          placeholder="Tìm trong văn bản... (Ctrl+F)" 
          bind:value={searchQuery}
          aria-label="Tìm kiếm trong văn bản"
        />
        <button class="icon-btn" onclick={() => showReplace = !showReplace} title="Tìm kiếm & Thay thế">
          🔄
        </button>
      </div>

      <div class="auto-scroll-toggle">
        <label title="Tự động cuộn đến đoạn đang phát audio">
          <input type="checkbox" bind:checked={uiState.autoScrollToPlaying} />
          Tự cuộn
        </label>
      </div>
    </div>
  </header>

  {#if showReplace}
    <div class="replace-bar">
      <input type="text" placeholder="Thay thế bằng..." bind:value={replaceText} aria-label="Chuỗi thay thế" />
      <button class="btn btn-secondary btn-sm" onclick={handleReplaceAll}>Thay thế tất cả</button>
    </div>
  {/if}

  <!-- Smart Error Recovery Banner (R3) -->
  {#if activeErrorSegment}
    <SmartErrorBanner
      {activeErrorSegment}
      onResynthesize={(seg) => handleResynthesizeSegment(seg)}
    />
  {/if}

  <!-- Offline Detection Banner -->
  {#if isOffline}
    <div class="offline-banner" role="alert">
      <span>⚡ Mất kết nối mạng — Không thể gọi Gemini API. Kiểm tra lại Wi-Fi hoặc Ethernet.</span>
    </div>
  {/if}

  <!-- Main Content Area -->
  <div class="editor-content-viewport">
      <!-- Segments List / Editor View -->
      <div class="segments-list" role="feed" aria-label="Danh sách đoạn văn bản">
        <!-- Batch Segment Action Bar -->
        <BatchActionsBar
          selectedCount={selectedSegmentIds.size}
          isAllSelected={isAllSegmentsSelected}
          onToggleSelectAll={toggleSelectAllSegments}
          onDeleteSelected={handleDeleteSelectedSegments}
        />

        <SegmentList
          {segments}
          {selectedSegmentIds}
          {isSynthesizingPreview}
          registerTextarea={(id, el) => {
            if (el) textareaElements[id] = el;
            else delete textareaElements[id];
          }}
          onSelectSegment={(id, e) => toggleSelectSegment(id, e)}
          onSegmentClick={(seg) => handleSegmentClick(seg)}
          onSegmentTextInput={(seg) => handleSegmentTextInput(seg)}
          onPlayPreview={(seg) => handlePlayPreview(seg)}
          onResynthesizeSegment={(seg) => handleResynthesizeSegment(seg)}
          onSplitSegment={(seg) => handleSplitSegment(seg)}
          onMoveSegment={(seg, dir) => handleMoveSegment(seg, dir)}
          onInsertSegmentBelow={(seg) => handleInsertSegmentBelow(seg)}
          onMergeWithPrevious={(seg) => handleMergeWithPrevious(seg)}
          onDeleteSingleSegment={(seg) => handleDeleteSingleSegment(seg)}
        />
      </div>
    </div>
</div>

<style>
  .editor-central-container {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--color-bg-surface);
    border-right: 1px solid var(--color-border);
    flex: 1;
    min-width: 480px;
  }

  .editor-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-3) var(--space-4);
    background: var(--color-bg-surface-raised);
    border-bottom: 1px solid var(--color-border);
  }

  .editor-tools {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
  }

  .action-icons-left {
    display: flex;
    gap: var(--space-1);
  }

  .action-btn-sm {
    background: var(--color-bg-surface-raised);
    border: 1px solid var(--color-border);
    transition: all 0.2s;
  }
  .action-btn-sm:hover {
    background: var(--color-bg-surface-hover);
    border-color: var(--color-border-focus);
  }

  .search-field {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .search-field input {
    height: var(--target-btn-sm);
    width: 200px;
  }

  .icon-btn {
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
  }
  .icon-btn:hover { background: var(--color-bg-surface-hover); }

  .auto-scroll-toggle {
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
  }

  .replace-bar {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    background: var(--color-bg-surface-hover);
    border-bottom: 1px solid var(--color-border);
  }

  .editor-content-viewport {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    padding: var(--space-4);
  }

  .segments-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  /* R4 Keyframe Animations & Styles */
  @keyframes pulse-ring {
    0% {
      box-shadow: 0 0 0 0 rgba(245, 158, 11, 0.4);
    }
    70% {
      box-shadow: 0 0 0 8px rgba(245, 158, 11, 0);
    }
    100% {
      box-shadow: 0 0 0 0 rgba(245, 158, 11, 0);
    }
  }

  .offline-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    background: rgba(239, 68, 68, 0.08);
    border: 1px solid rgba(239, 68, 68, 0.25);
    border-radius: var(--radius-md);
    padding: 8px 16px;
    margin: var(--space-2) var(--space-4) 0 var(--space-4);
    font-size: var(--font-size-xs);
    color: #b91c1c;
    font-weight: 500;
  }
</style>
