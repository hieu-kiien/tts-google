<script lang="ts">
  let {
    models = [],
    voices = [],
    presets = [],
    chunkModes = [],
    selectedModel = $bindable("gemini-3.1-flash-tts-preview"),
    selectedVoice = $bindable("Kore"),
    selectedPreset = $bindable("Tự nhiên"),
    selectedChunkMode = $bindable("auto"),
    speakingRate = $bindable(1.0),
    pitchShift = $bindable(0.0),
    volumeGainDb = $bindable(0.0),
    audioFormat = $bindable("wav"),
    sampleRateHz = $bindable("24000"),
    silenceGapMs = $bindable(300),
  }: {
    models: { id: string; label: string }[];
    voices: { id: string; label: string }[];
    presets: string[];
    chunkModes: { id: string; label: string }[];
    selectedModel: string;
    selectedVoice: string;
    selectedPreset: string;
    selectedChunkMode: string;
    speakingRate: number;
    pitchShift: number;
    volumeGainDb: number;
    audioFormat: string;
    sampleRateHz: string;
    silenceGapMs: number;
  } = $props();
</script>

<aside class="control-panel">
  <h3>Google Speech Studio Settings</h3>

  <div class="form-group">
    <label for="model-sel">Chọn Gemini TTS Model:</label>
    <select id="model-sel" bind:value={selectedModel} class="select-input highlight-select">
      {#each models as m}
        <option value={m.id}>{m.label}</option>
      {/each}
    </select>
  </div>

  <div class="form-group">
    <label for="voice-sel">Chọn Giọng Đọc Mặc Định ({voices.length} Voices):</label>
    <select id="voice-sel" bind:value={selectedVoice} class="select-input">
      {#each voices as v}
        <option value={v.id}>{v.label}</option>
      {/each}
    </select>
  </div>

  <div class="form-group">
    <label for="preset-sel">Style Preset (Phong Cách):</label>
    <select id="preset-sel" bind:value={selectedPreset} class="select-input">
      {#each presets as p}
        <option value={p}>{p}</option>
      {/each}
    </select>
  </div>

  <div class="form-group">
    <label for="chunk-sel">Phương Thức Chia Đoạn:</label>
    <select id="chunk-sel" bind:value={selectedChunkMode} class="select-input highlight-select">
      {#each chunkModes as cm}
        <option value={cm.id}>{cm.label}</option>
      {/each}
    </select>
  </div>

  <div class="form-group">
    <div class="label-row">
      <label for="rate-range">Tốc Độ Đọc (Speaking Rate):</label>
      <span class="val-badge">{speakingRate}x</span>
    </div>
    <input id="rate-range" type="range" min="0.5" max="2.0" step="0.1" bind:value={speakingRate} />
  </div>

  <div class="form-group">
    <div class="label-row">
      <label for="pitch-range">Cao Độ (Pitch Shift):</label>
      <span class="val-badge">{pitchShift > 0 ? "+" : ""}{pitchShift} st</span>
    </div>
    <input id="pitch-range" type="range" min="-6.0" max="6.0" step="0.5" bind:value={pitchShift} />
  </div>

  <div class="form-group">
    <div class="label-row">
      <label for="vol-range">Âm Lượng (Volume Gain):</label>
      <span class="val-badge">{volumeGainDb > 0 ? "+" : ""}{volumeGainDb} dB</span>
    </div>
    <input id="vol-range" type="range" min="-12.0" max="6.0" step="1.0" bind:value={volumeGainDb} />
  </div>

  <div class="form-row">
    <div class="form-group flex-1">
      <label for="fmt-sel">Định Dạng:</label>
      <select id="fmt-sel" bind:value={audioFormat} class="select-input">
        <option value="wav">WAV (PCM)</option>
        <option value="mp3">MP3</option>
      </select>
    </div>
    <div class="form-group flex-1">
      <label for="rate-sel">Sample Rate:</label>
      <select id="rate-sel" bind:value={sampleRateHz} class="select-input">
        <option value="24000">24.000 Hz</option>
        <option value="16000">16.000 Hz</option>
        <option value="44100">44.100 Hz</option>
      </select>
    </div>
  </div>

  <div class="form-group">
    <label for="silence-input">Khoảng nghỉ giữa các đoạn (ms):</label>
    <input id="silence-input" type="number" min="0" max="5000" step="50" bind:value={silenceGapMs} class="text-input" />
  </div>

  <div class="shortcuts-card">
    <h4>⚡ Phím Tắt Đã Kích Hoạt</h4>
    <ul>
      <li><code>Ctrl + N</code>: Tạo dự án mới</li>
      <li><code>Ctrl + Enter</code>: Chia đoạn văn bản</li>
      <li><code>Space</code>: Play/Pause Audio</li>
    </ul>
  </div>
</aside>

<style>
  .control-panel {
    width: 320px;
    background: var(--color-bg-surface);
    border-left: 1px solid var(--color-border);
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
  }
  .control-panel h3 {
    font-size: 15px;
    font-weight: 700;
    margin-bottom: 4px;
    color: var(--color-text-primary);
  }
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-group label {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-secondary);
  }
  .select-input, .text-input {
    width: 100%;
    padding: 8px 10px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    font-size: 13px;
  }
  .highlight-select {
    border-color: var(--color-accent);
    background: var(--color-bg-surface-selected);
  }
  .label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .val-badge {
    font-size: 12px;
    font-weight: 700;
    color: var(--color-accent);
  }
  .form-row {
    display: flex;
    gap: 10px;
  }
  .flex-1 { flex: 1; }
  .shortcuts-card {
    background: var(--color-bg-surface-raised);
    border: 1px dashed var(--color-border);
    border-radius: 6px;
    padding: 12px;
    font-size: 12px;
    margin-top: 10px;
  }
  .shortcuts-card h4 {
    margin: 0 0 6px 0;
    font-size: 13px;
    color: var(--color-text-secondary);
  }
  .shortcuts-card ul {
    margin: 0;
    padding-left: 18px;
    color: var(--color-text-muted);
  }
  .shortcuts-card code {
    background: var(--color-border);
    padding: 2px 4px;
    border-radius: 4px;
    font-size: 11px;
  }
</style>
