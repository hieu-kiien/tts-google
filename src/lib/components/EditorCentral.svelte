<script lang="ts">
  import { projectState } from "../state/projectState.svelte";
  import { uiState } from "../state/uiState.svelte";
  import { playerState } from "../state/playerState.svelte";
  import type { SegmentRecord } from "../types/tts";
  import { invoke } from "@tauri-apps/api/core";
  import { toastStore } from "../state/toasts.svelte";
  import { enqueueProject, pauseProject, resumeProject } from "../api/queueClient";
  import { getErrorMessage } from "../utils/errorUtils";
  import { onMount } from "svelte";

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
    if (projectState.segments.length > 0) return projectState.segments;
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
    seg.audio_path = undefined;
    seg.status = 'processing';
    seg.error_message = undefined;
    seg.error_code = undefined;
    seg.last_error_message = undefined;

    toastStore.showInfo(`Đang tạo lại âm thanh cho đoạn #${seg.position}...`);
    try {
      const res = await invoke<{ data_url: string; duration_ms: number }>(
        "synthesize_preview_audio",
        {
          text: seg.spoken_text || seg.text,
          voice: seg.voice || projectState.currentProject?.voice || "Kore",
          model: projectState.currentProject?.model || "gemini-3.1-flash-tts-preview",
          speed: 1.0,
          pitch: 1.0
        }
      );
      seg.status = 'success';
      seg.duration_ms = res.duration_ms;
      playerState.playUrl(res.data_url, seg.id);
      toastStore.showSuccess(`Đã tạo lại audio đoạn #${seg.position} thành công (${(res.duration_ms / 1000).toFixed(1)}s)!`);
    } catch (err: unknown) {
      seg.status = 'failed';
      seg.error_message = getErrorMessage(err);
      toastStore.showError(`Lỗi tạo lại đoạn #${seg.position}: ${getErrorMessage(err)}`);
    }
  }

  const previewAudioCache = new Map<string, { data_url: string; duration_ms: number }>();
  let isRechunking = $state(false);

  async function handleRechunkAllText() {
    if (!confirm("CẢNH BÁO: Hành động này sẽ chia lại toàn bộ văn bản và ghi đè lên các đoạn hiện tại. Các chỉnh sửa thủ công của bạn sẽ bị mất. Bạn có chắc chắn muốn tiếp tục?")) {
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
        const updatedSegs = await invoke<SegmentRecord[]>("rechunk_project_segments", {
          projectId: proj.id,
          sourceText: textToChunk,
          mode: "auto"
        });
        projectState.segments = updatedSegs;
        toastStore.showSuccess(`Đã tự động chia nhỏ thành ${updatedSegs.length} đoạn audio (30-60s)!`);
      } else {
        const chunks = await invoke<any[]>("chunk_text_preview", { text: textToChunk, mode: "auto" });
        const newSegs: SegmentRecord[] = chunks.map((c: any, idx: number) => ({
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
      
      const res = await invoke<{ data_url: string; duration_ms: number; sample_rate: number }>(
        "synthesize_preview_audio",
        {
          text: textToSynthesize,
          voice,
          model: projectState.currentProject?.model || "gemini-3.1-flash-tts-preview",
          speed: 1.0,
          pitch: 1.0
        }
      );

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
      toastStore.showInfo(`Đang tách đoạn #${seg.position} tại vị trí ký tự ${splitIndex}...`);
      await invoke("split_segment", {
        projectId: projectId,
        segmentId: seg.id,
        splitIndex: splitIndex
      });
      
      const updatedSegs = await invoke<SegmentRecord[]>("get_project_segments", {
        projectId: projectId
      });
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
      const updated = await invoke<SegmentRecord[]>("get_project_segments", { projectId });
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
      toastStore.showInfo(`Đang gộp đoạn #${seg.position} với đoạn #${seg.position - 1}...`);
      await invoke("merge_segments", {
        projectId: projectId,
        segmentId: seg.id
      });
      const updatedSegs = await invoke<SegmentRecord[]>("get_project_segments", {
        projectId: projectId
      });
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
      toastStore.showInfo(`Đang xóa đoạn #${seg.position}...`);
      await invoke("delete_segment", { projectId, segmentId: seg.id });
      const updated = await invoke<SegmentRecord[]>("get_project_segments", { projectId });
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
      toastStore.showInfo(`Đang xóa ${ids.length} đoạn đã chọn...`);
      await invoke("delete_segments_batch", { projectId, segmentIds: ids });
      const updated = await invoke<SegmentRecord[]>("get_project_segments", { projectId });
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
      await invoke("move_segment", { projectId, segmentId: seg.id, direction });
      const updated = await invoke<SegmentRecord[]>("get_project_segments", { projectId });
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
      await invoke("insert_segment_at", { projectId, position: targetPos, text: "Đoạn văn bản mới..." });
      const updated = await invoke<SegmentRecord[]>("get_project_segments", { projectId });
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
    <div class="smart-error-banner" role="alert">
      <div class="error-info">
        <span class="error-icon">⚠️</span>
        <div class="error-text">
          <strong>Sự cố API ({activeErrorSegment.error_code || '429'}):</strong> 
          Đoạn #{activeErrorSegment.position} {activeErrorSegment.error_message || activeErrorSegment.last_error_message || 'gặp sự cố khi kết nối Gemini API (Quá giới hạn quota hoặc API key không hợp lệ).'}
        </div>
      </div>
      <div class="error-actions">
        <button class="btn-banner secondary" onclick={() => uiState.showApiKeyModal = true}>
          🔑 Đổi API Key & Tiếp Tục
        </button>
        <button class="btn-banner primary" onclick={() => handleResynthesizeSegment(activeErrorSegment)}>
          🔄 Thử Lại Đoạn #{activeErrorSegment.position}
        </button>
        <button class="btn-banner" style="background: var(--color-success-bg); color: var(--color-success-text); border: 1px solid var(--color-success-border);" onclick={async () => {
          if (projectState.currentProject?.id) {
            try {
              await resumeProject(projectState.currentProject.id);
              toastStore.showSuccess("Đã tiếp tục chạy hàng đợi.");
            } catch (err) {
              toastStore.showError(getErrorMessage(err));
            }
          }
        }}>
          ▶ Tiếp Tục Hàng Đợi
        </button>
      </div>
    </div>
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
        {#if selectedSegmentIds.size > 0}
          <div class="segment-batch-toolbar active">
            <label class="select-all-segments-label">
              <input type="checkbox" checked={isAllSegmentsSelected} onchange={toggleSelectAllSegments} />
              Chọn tất cả
            </label>
            <div class="batch-segment-actions">
              <span class="selected-count">Đã chọn <strong>{selectedSegmentIds.size}</strong> đoạn</span>
              <button class="btn btn-danger btn-sm" onclick={handleDeleteSelectedSegments}>
                🗑️ Xóa hàng loạt
              </button>
            </div>
          </div>
        {/if}

        {#each segments as seg (seg.id)}
          <div 
            id={`seg_card_${seg.id}`}
            class="segment-row {selectedSegmentIds.has(seg.id) ? 'selected-batch' : ''} {projectState.activeSegmentId === seg.id ? 'selected' : ''} {playerState.currentPlayingSegmentId === seg.id ? 'playing' : ''} {seg.status === 'processing' || seg.status === 'queued' ? 'processing-active' : ''} {seg.is_locked ? 'locked' : ''} {seg.is_skipped ? 'skipped' : ''}"
            onclick={() => handleSegmentClick(seg)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleSegmentClick(seg); }}
            role="button"
            tabindex="0"
            aria-label={`Đoạn ${seg.position}, trạng thái ${seg.status}`}
          >
            <!-- Line Number & Indicators -->
            <div class="segment-meta">
              <input 
                type="checkbox" 
                class="seg-checkbox"
                checked={selectedSegmentIds.has(seg.id)} 
                onclick={(e) => toggleSelectSegment(seg.id, e)}
                title="Chọn đoạn này"
              />
              <span class="seg-num">#{seg.position}</span>
              {#if seg.is_locked}
                <span class="status-icon" title="Đoạn bị khóa">🔒</span>
              {/if}
              {#if seg.is_skipped}
                <span class="status-icon" title="Đã đánh dấu bỏ qua">⏭️</span>
              {/if}
              {#if seg.text.length > 700}
                <span class="warning-badge" title="Đoạn quá dài (> 700 ký tự). Bạn có thể bấm nút ✂️ Tách đoạn.">⚠️ Quá dài</span>
              {/if}
            </div>

            <!-- Segment Text Display / Editor -->
            <div class="segment-body">
                <textarea 
                  id={`textarea-${seg.id}`}
                  bind:this={textareaElements[seg.id]}
                  class="segment-textarea" 
                  bind:value={seg.text}
                  oninput={() => handleSegmentTextInput(seg)}
                  aria-label={`Nội dung đoạn ${seg.position}`}
                ></textarea>

              <div class="segment-footer-info">
                <span>{seg.text.length} ký tự</span>
                {#if seg.duration_ms > 0}
                  <span class="duration-badge">⏱️ {(seg.duration_ms / 1000).toFixed(1)}s</span>
                {/if}

                {#if seg.status === 'approved'}
                  <span class="status-approved" title="Đã duyệt">⭐</span>
                {:else if seg.status === 'success'}
                  <span class="status-success" title="Đã tạo âm thanh">🟢</span>
                {:else if seg.status === 'processing' || seg.status === 'queued'}
                  <div class="status-processing-box">
                    <div class="sound-wave-anim" title="Đang tổng hợp âm thanh với Gemini API">
                      <span class="bar bar-1"></span>
                      <span class="bar bar-2"></span>
                      <span class="bar bar-3"></span>
                    </div>
                  </div>
                {:else if seg.status === 'failed'}
                  <span class="status-error" title="Lỗi API">🔴</span>
                {:else if seg.status === 'retry_wait'}
                  <span class="status-warning" title="Đang chờ thử lại">🔄</span>
                {:else if seg.status === 'stale'}
                  <span class="status-warning badge-stale" title="Văn bản đã thay đổi, cần tạo lại âm thanh">⚠️</span>
                {:else}
                  <span class="status-pending" title="Chưa tạo âm thanh">⚪</span>
                {/if}
              </div>
            </div>

            <!-- Segment Quick Actions -->
            <div class="segment-actions">
              <button 
                class="btn-action" 
                onclick={(e) => { e.stopPropagation(); handlePlayPreview(seg); }}
                disabled={isSynthesizingPreview}
                title="Nghe thử đoạn này"
                aria-label="Nghe thử"
              >
                ▶
              </button>
              {#if seg.status === 'stale'}
                <button 
                  class="btn-action btn-resynthesize" 
                  onclick={(e) => { e.stopPropagation(); handleResynthesizeSegment(seg); }}
                  title="Tạo lại đoạn này ngay"
                  aria-label="Tạo lại đoạn này"
                >
                  ⚡
                </button>
              {/if}
              <button 
                class="btn-action btn-secondary-action" 
                onclick={(e) => { e.stopPropagation(); handleSplitSegment(seg); }}
                title="Tách đoạn tại vị trí con trỏ"
                aria-label="Tách đoạn"
              >
                ✂️
              </button>
              {#if seg.position > 1}
                <button 
                  class="btn-action btn-secondary-action" 
                  onclick={(e) => { e.stopPropagation(); handleMoveSegment(seg, 'up'); }}
                  title="Di chuyển lên trên"
                  aria-label="Di chuyển lên"
                >
                  ⬆️
                </button>
              {/if}
              {#if seg.position < segments.length}
                <button 
                  class="btn-action btn-secondary-action" 
                  onclick={(e) => { e.stopPropagation(); handleMoveSegment(seg, 'down'); }}
                  title="Di chuyển xuống dưới"
                  aria-label="Di chuyển xuống"
                >
                  ⬇️
                </button>
              {/if}
              <button 
                class="btn-action btn-secondary-action" 
                onclick={(e) => { e.stopPropagation(); handleInsertSegmentBelow(seg); }}
                title="Chèn thêm đoạn mới bên dưới"
                aria-label="Chèn đoạn mới"
              >
                ➕
              </button>
              {#if seg.position > 1}
                <button 
                  class="btn-action btn-merge btn-secondary-action" 
                  onclick={(e) => { e.stopPropagation(); handleMergeWithPrevious(seg); }}
                  title="Gộp với đoạn #{seg.position - 1} phía trên"
                  aria-label="Gộp đoạn"
                >
                  🔗
                </button>
              {/if}
              <button 
                class="btn-action btn-delete-segment" 
                onclick={(e) => { e.stopPropagation(); handleDeleteSingleSegment(seg); }}
                title="Xóa đoạn này"
                aria-label="Xóa đoạn"
              >
                🗑️
              </button>
            </div>
          </div>
        {/each}
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

  /* R3 Smart Error Recovery Banner Styling */
  .smart-error-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: var(--radius-md);
    padding: 10px 16px;
    margin: var(--space-3) var(--space-4) 0 var(--space-4);
  }

  .error-info {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: var(--font-size-xs);
    color: #b91c1c;
  }

  .error-icon {
    font-size: 18px;
  }

  .error-actions {
    display: flex;
    gap: 8px;
  }

  .btn-banner {
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    font-size: 12px;
    font-weight: 600;
    border: none;
    cursor: pointer;
  }

  .btn-banner.primary {
    background: #ef4444;
    color: white;
  }

  .btn-banner.secondary {
    background: #ffffff;
    color: #374151;
    border: 1px solid #d1d5db;
  }

  .segment-batch-toolbar.active {
    position: sticky;
    top: 0;
    z-index: 10;
    background: var(--color-bg-surface-raised);
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--color-accent);
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: var(--space-2) var(--space-3);
    box-shadow: var(--shadow-md);
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
    overflow-y: auto;
    padding: var(--space-4);
  }

  .segments-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .segment-row {
    display: flex;
    gap: var(--space-3);
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-surface);
    transition: background 0.15s ease, border-color 0.15s ease;
  }

  .segment-row.selected {
    border-color: var(--color-accent);
    background: var(--color-bg-surface-selected);
  }

  .segment-row.playing {
    border-color: var(--color-success);
    background: var(--color-success-bg);
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

  @keyframes wave-bar {
    0%, 100% { transform: scaleY(0.3); }
    50% { transform: scaleY(1.0); }
  }

  .segment-row.processing-active {
    border-color: #f59e0b;
    animation: pulse-ring 1.8s infinite;
    background: rgba(245, 158, 11, 0.06);
  }

  .status-processing-box {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .sound-wave-anim {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 14px;
  }

  .sound-wave-anim .bar {
    width: 3px;
    height: 100%;
    background-color: #d97706;
    border-radius: 2px;
    animation: wave-bar 0.7s ease-in-out infinite alternate;
  }

  .sound-wave-anim .bar-1 { animation-delay: 0s; }
  .sound-wave-anim .bar-2 { animation-delay: 0.2s; }
  .sound-wave-anim .bar-3 { animation-delay: 0.4s; }

  .segment-row.locked { opacity: 0.8; }
  .segment-row.skipped { text-decoration: line-through; opacity: 0.6; }

  .segment-meta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
    min-width: 44px;
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
  }

  .warning-badge {
    color: var(--color-warning-text);
    background: var(--color-warning-bg);
    padding: 2px 4px;
    border-radius: var(--radius-sm);
    font-size: 10px;
  }

  .segment-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .segment-textarea {
    width: 100%;
    min-height: 52px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--color-text-primary);
    font-size: var(--font-size-base);
    line-height: 1.6;
    font-family: inherit;
    resize: vertical;
    field-sizing: content;
    outline: none;
  }

  .segment-footer-info {
    display: flex;
    gap: var(--space-4);
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
    align-items: center;
  }

  .status-approved { color: var(--color-accent-text); font-weight: 600; }
  .status-success { color: var(--color-success-text); font-weight: 500; }
  .status-warning { color: var(--color-warning-text); font-weight: 500; }
  .status-error { color: var(--color-error-text); font-weight: 500; }
  .status-pending { color: var(--color-text-muted); }

  .btn-resynthesize {
    color: #d97706 !important;
    background: rgba(245, 158, 11, 0.15) !important;
    font-weight: bold;
  }
  .btn-resynthesize:hover {
    background: rgba(245, 158, 11, 0.3) !important;
  }
  .badge-stale {
    background: rgba(245, 158, 11, 0.15);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
  }
  .duration-badge {
    color: var(--color-text-muted);
    font-weight: 500;
  }

  .segment-actions {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .btn-action {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--color-bg-surface-raised);
  }
  .btn-action:hover { background: var(--color-bg-surface-hover); }

  .btn-secondary-action {
    display: none;
  }

  .segment-row:hover .btn-secondary-action,
  .segment-row.selected .btn-secondary-action {
    display: flex;
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

  .btn-merge {
    color: #2563eb !important;
    background: rgba(37, 99, 235, 0.1) !important;
  }
  .btn-merge:hover {
    background: rgba(37, 99, 235, 0.2) !important;
  }

  .btn-delete-segment {
    color: #dc2626 !important;
  }
  .btn-delete-segment:hover {
    background: rgba(220, 38, 38, 0.15) !important;
  }

  .segment-batch-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 14px;
    background: var(--color-bg-surface-raised);
    border-bottom: 1px solid var(--color-border);
    font-size: 13px;
  }

  .select-all-segments-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-weight: 500;
  }

  .batch-segment-actions {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .seg-checkbox {
    width: 16px;
    height: 16px;
    cursor: pointer;
    margin-right: 4px;
  }

  .segment-row.selected-batch {
    background: rgba(37, 99, 235, 0.05);
    border-left: 3px solid var(--color-accent);
  }
</style>
