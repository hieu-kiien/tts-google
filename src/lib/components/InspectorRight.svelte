<script lang="ts">
  import { projectState } from "../state/projectState.svelte";
  import { uiState } from "../state/uiState.svelte";
  import { playerState } from "../state/playerState.svelte";
  import { toastStore } from "../state/toasts.svelte";
  import type { SegmentIssue } from "../types/tts";
  import { getErrorMessage } from "../utils/errorUtils";
  import { updateSegmentVoice, updateProjectVoice, updateSegmentText } from "../api/projectClient";
  import { synthesizePreviewAudio } from "../api/audioClient";

  let voiceSearch = $state("");
  let customTestText = $state("");
  let scopeSelection = $state<"segment" | "selected" | "chapter" | "project">("segment");

  let speakingRate = $state(1.0);
  let expressiveness = $state(0.8);
  let formality = $state("bình thường");
  let previewingVoiceId = $state<string | null>(null);

  const voices = [
    { id: "Kore", name: "Kore", tags: ["Nữ", "Tự nhiên", "Trầm ấm"], desc: "Thích hợp đọc sách nói, tiểu thuyết" },
    { id: "Aoede", name: "Aoede", tags: ["Nữ", "Truyền cảm", "Sâu lắng"], desc: "Phù hợp thơ ca, tản văn, tài liệu cảm xúc" },
    { id: "Zephyr", name: "Zephyr", tags: ["Nữ", "Nhẹ nhàng", "Bình tĩnh"], desc: "Lý tưởng cho hướng dẫn thiền, podcast" },
    { id: "Puck", name: "Puck", tags: ["Nam", "Năng động", "Trẻ trung"], desc: "Phù hợp video ngắn, quảng cáo, tin tức trẻ" },
    { id: "Charon", name: "Charon", tags: ["Nam", "Trang trọng", "Đọc tin"], desc: "Giọng thời sự chuẩn, bản tin chính luận" },
    { id: "Fenrir", name: "Fenrir", tags: ["Nam", "Mạnh mẽ", "Cuốn hút"], desc: "Đọc sách kỹ năng, tự truyện, thuyết minh" },
  ];

  const filteredVoices = $derived.by(() => {
    if (!voiceSearch.trim()) return voices;
    const query = voiceSearch.toLowerCase();
    return voices.filter(v => 
      v.name.toLowerCase().includes(query) || 
      v.desc.toLowerCase().includes(query) ||
      v.tags.some(t => t.toLowerCase().includes(query))
    );
  });

  const activeSegment = $derived(projectState.activeSegment);

  async function handleSelectVoice(voiceId: string) {
    if (scopeSelection === "segment" && activeSegment && projectState.currentProject) {
      activeSegment.voice = voiceId;
      if (activeSegment.audio_path) activeSegment.status = 'stale';
      try {
        await updateSegmentVoice(projectState.currentProject.id, activeSegment.id, voiceId);
        toastStore.showSuccess(`Đã gán giọng đọc ${voiceId} cho Đoạn #${activeSegment.position}`);
      } catch (err: unknown) {
        toastStore.showError(`Lỗi lưu giọng đọc segment: ${getErrorMessage(err)}`);
      }
    } else if (projectState.currentProject) {
      projectState.currentProject.voice = voiceId;
      for (const seg of projectState.segments) {
        if (seg.audio_path && (seg.status === 'success' || seg.status === 'approved')) {
          seg.status = 'stale';
        }
      }
      try {
        await updateProjectVoice(projectState.currentProject.id, voiceId);
        toastStore.showSuccess(`Đã lưu giọng đọc toàn bộ dự án: ${voiceId}`);
      } catch (err: unknown) {
        console.warn("Lỗi lưu giọng đọc vào DB:", getErrorMessage(err));
      }
    }
  }

  const voiceSampleCache = new Map<string, string>();

  const voiceSampleTexts: Record<string, string> = {
    Kore: "Xin chào quý vị, đây là mẫu giọng đọc Kore tự nhiên, truyền cảm, thích hợp đọc sách nói.",
    Aoede: "Xin chào quý vị, tôi là giọng đọc Aoede sâu lắng và mượt mà.",
    Zephyr: "Xin chào bạn, tôi là giọng đọc Zephyr nhẹ nhàng, bình tĩnh.",
    Puck: "Xin chào! Đây là mẫu giọng đọc Puck trẻ trung, năng động và đầy năng lượng.",
    Charon: "Xin chào quý khán giả, tôi là giọng đọc Charon trang trọng, chuẩn bản tin thời sự.",
    Fenrir: "Xin chào các bạn, tôi là giọng đọc Fenrir mạnh mẽ, cuốn hút cho tự truyện và thuyết minh."
  };

  async function handlePreviewVoice(voiceId: string) {
    if (voiceSampleCache.has(voiceId)) {
      const dataUrl = voiceSampleCache.get(voiceId)!;
      playerState.playUrl(dataUrl, `preview_voice_${voiceId}`);
      toastStore.showSuccess(`Phát mẫu giọng ${voiceId} (từ bộ nhớ đệm ⚡)`);
      return;
    }

    try {
      previewingVoiceId = voiceId;
      toastStore.showInfo(`Đang nạp mẫu giọng đọc ${voiceId}...`);
      const previewText = customTestText.trim() || voiceSampleTexts[voiceId] || "Xin chào! Đây là mẫu giọng đọc tiếng Việt chất lượng cao từ Gemini API.";
      
      const res = await synthesizePreviewAudio({
        voice: voiceId,
        text: previewText,
        model: projectState.currentProject?.model || "gemini-3.1-flash-tts-preview",
        speed: speakingRate,
        pitch: 1.0
      });

      voiceSampleCache.set(voiceId, res.data_url);
      playerState.playUrl(res.data_url, `preview_voice_${voiceId}`);
      toastStore.showSuccess(`Đã nạp mẫu giọng ${voiceId} (${(res.duration_ms / 1000).toFixed(1)}s)`);
    } catch (err: unknown) {
      toastStore.showError(`Lỗi nghe thử giọng ${voiceId}: ${getErrorMessage(err)}`);
    } finally {
      previewingVoiceId = null;
    }
  }

  async function handleSaveSynthesisSettings() {
    if (!projectState.currentProject) return;
    try {
      projectState.currentProject.pacing = `Tốc độ: ${speakingRate}x, Biểu cảm: ${Math.round(expressiveness * 100)}%, Phong cách: ${formality}`;
      
      if (projectState.currentProject.id && projectState.currentProject.voice) {
        try {
          await updateProjectVoice(projectState.currentProject.id, projectState.currentProject.voice);
        } catch (err: unknown) {
          console.warn("Lỗi update_project_voice:", getErrorMessage(err));
        }
      }

      if (activeSegment) {
        activeSegment.prompt = `[Speed: ${speakingRate}x, Expressiveness: ${expressiveness}, Formality: ${formality}] ${activeSegment.prompt || 'Đọc tự nhiên, truyền cảm'}`;
        if (projectState.currentProject.id) {
          try {
            await updateSegmentText(projectState.currentProject.id, activeSegment.id, activeSegment.text);
          } catch {
            // ignore if project draft
          }
        }
      }
      toastStore.showSuccess("Đã lưu cài đặt tốc độ & biểu cảm giọng đọc!");
    } catch (err: unknown) {
      toastStore.showError(`Lỗi lưu cài đặt phong cách: ${getErrorMessage(err)}`);
    }
  }

  function handleReportIssue(issue: SegmentIssue) {
    if (!activeSegment) return;
    projectState.setSegmentReviewStatus(activeSegment.id, "needs_fix", issue);
    toastStore.showInfo(`Đã đánh dấu sự cố "${issue}" cho đoạn #${activeSegment.position}. Hệ thống đề xuất: Thêm vào từ điển hoặc chỉnh sửa văn bản sẽ đọc.`);
  }

  function handleAddWordToDict() {
    uiState.showDictionaryModal = true;
  }
</script>

<aside class="inspector-container" aria-label="Khung điều khiển Inspector">
  <!-- Inspector Sub-Tabs -->
  <div class="inspector-tabs" role="tablist">
    <button 
      class="tab-item {uiState.inspectorTab === 'voice' ? 'active' : ''}"
      onclick={() => uiState.inspectorTab = 'voice'}
      role="tab"
    >
      🎙️ Giọng
    </button>
    <button 
      class="tab-item {uiState.inspectorTab === 'style' ? 'active' : ''}"
      onclick={() => uiState.inspectorTab = 'style'}
      role="tab"
    >
      🎛️ Phong cách
    </button>
  </div>

  <div class="inspector-content">
    {#if uiState.inspectorTab === 'voice'}
      <!-- Voice Picker Section (Section 5.4) -->
      <div class="voice-picker-section">
        <h3>Bộ chọn giọng đọc Gemini</h3>
        
        <input 
          type="text" 
          placeholder="Tìm giọng (Trầm, Nữ, Đọc tin...)" 
          bind:value={voiceSearch}
          aria-label="Tìm kiếm giọng đọc"
        />

        <div class="custom-test-box">
          <label for="custom-preview-input">Văn bản nghe thử tùy chọn:</label>
          <input 
            id="custom-preview-input"
            type="text" 
            placeholder="Nhập câu thử giọng của riêng bạn..." 
            bind:value={customTestText}
            aria-label="Văn bản nghe thử tùy chọn"
          />
        </div>

        <div class="voices-list">
          {#each filteredVoices as v (v.id)}
            <div 
              class="voice-card {((scopeSelection === 'segment' && activeSegment?.voice) ? activeSegment.voice : projectState.currentProject?.voice) === v.id ? 'selected' : ''}"
              onclick={() => handleSelectVoice(v.id)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleSelectVoice(v.id); }}
              role="button"
              tabindex="0"
              aria-label={`Giọng ${v.name}, ${v.desc}`}
            >
              <div class="voice-card-header">
                <strong>{v.name}</strong>
                <button 
                  class="btn-preview-sm"
                  disabled={previewingVoiceId === v.id}
                  onclick={(e) => { e.stopPropagation(); handlePreviewVoice(v.id); }}
                  title="Nghe thử mẫu giọng"
                  aria-label={`Nghe mẫu giọng ${v.name}`}
                >
                  {previewingVoiceId === v.id ? '⏳ Đang tạo...' : '▶ Nghe mẫu'}
                </button>
              </div>
              <p class="voice-desc">{v.desc}</p>
              <div class="voice-tags">
                {#each v.tags as t}
                  <span class="tag">{t}</span>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </div>

    {:else if uiState.inspectorTab === 'style'}
      <!-- Style & Prompt Tuning (Section 5.5) -->
      <div class="style-section">
        <h3>Điều chỉnh phong cách giọng</h3>

        <div class="control-group">
          <label for="speed-range">Tốc độ đọc ({speakingRate}x)</label>
          <input id="speed-range" type="range" min="0.5" max="2.0" step="0.1" bind:value={speakingRate} onchange={handleSaveSynthesisSettings} />
        </div>

        <div class="control-group">
          <label for="express-range">Mức biểu cảm cảm xúc ({Math.round(expressiveness * 100)}%)</label>
          <input id="express-range" type="range" min="0.0" max="1.0" step="0.1" bind:value={expressiveness} onchange={handleSaveSynthesisSettings} />
        </div>

        <div class="control-group">
          <label for="formality-select">Độ trang trọng</label>
          <select id="formality-select" bind:value={formality} onchange={handleSaveSynthesisSettings}>
            <option value="trang trọng">Trang trọng / Tin tức</option>
            <option value="bình thường">Bình thường / Thường ngày</option>
            <option value="thân mật">Thân mật / Kể chuyện</option>
          </select>
        </div>

        <div class="scope-box">
          <label for="scope-opt-1"><strong>Áp dụng cho:</strong></label>
          <label id="scope-opt-1"><input type="radio" name="scope" value="segment" bind:group={scopeSelection} /> Đoạn hiện tại (#{activeSegment?.position || 1})</label>
          <label><input type="radio" name="scope" value="selected" bind:group={scopeSelection} /> Các đoạn đã chọn</label>
          <label><input type="radio" name="scope" value="chapter" bind:group={scopeSelection} /> Chương hiện tại</label>
          <label><input type="radio" name="scope" value="project" bind:group={scopeSelection} /> Toàn bộ dự án</label>
        </div>

        <div style="margin-top: 12px;">
          <button class="btn btn-primary" style="width: 100%;" onclick={handleSaveSynthesisSettings}>
            💾 Lưu cài đặt phong cách
          </button>
        </div>
      </div>
    {/if}
  </div>
</aside>

<style>
  .inspector-container {
    width: var(--inspector-width);
    height: 100%;
    background: var(--color-bg-surface);
    border-left: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
  }

  .inspector-tabs {
    display: flex;
    background: var(--color-bg-surface-raised);
    border-bottom: 1px solid var(--color-border);
  }

  .tab-item {
    flex: 1;
    height: var(--target-btn-md);
    font-size: var(--font-size-xs);
    font-weight: 500;
    color: var(--color-text-secondary);
  }
  .tab-item.active {
    background: var(--color-bg-surface);
    color: var(--color-accent);
    border-bottom: 2px solid var(--color-accent);
    font-weight: 600;
  }

  .inspector-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  h3 { font-size: var(--font-size-base); color: var(--color-text-primary); }

  .voices-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }

  .voice-card {
    padding: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    background: var(--color-bg-surface);
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .voice-card.selected {
    border-color: var(--color-accent);
    background: var(--color-bg-surface-selected);
  }

  .voice-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .btn-preview-sm {
    font-size: var(--font-size-xs);
    color: var(--color-accent);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    background: var(--color-accent-subtle);
  }

  .voice-desc { font-size: var(--font-size-xs); color: var(--color-text-muted); }

  .voice-tags { display: flex; gap: 4px; flex-wrap: wrap; }
  .tag { font-size: 10px; padding: 2px 6px; border-radius: var(--radius-full); background: var(--color-bg-surface-hover); }

  .control-group {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--font-size-xs);
  }

  .scope-box {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    background: var(--color-bg-surface-raised);
    padding: var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--font-size-xs);
    margin-top: var(--space-3);
  }

  .btn {
    height: var(--target-btn-md);
    border-radius: var(--radius-md);
    font-size: var(--font-size-xs);
  }
  .btn-primary { background: var(--color-accent); color: white; }
</style>
