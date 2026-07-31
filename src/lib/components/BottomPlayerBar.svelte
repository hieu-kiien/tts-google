<script lang="ts">
  import { playerState } from "../state/playerState.svelte";
  import { projectState } from "../state/projectState.svelte";

  function formatTime(seconds: number): string {
    if (!seconds || isNaN(seconds)) return "0:00";
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return `${mins}:${secs < 10 ? '0' : ''}${secs}`;
  }

  let isDragging = $state(false);
  let localTime = $state(0);

  function handleSliderInput(e: Event) {
    isDragging = true;
    localTime = parseFloat((e.target as HTMLInputElement).value);
  }

  function handleSliderChange(e: Event) {
    isDragging = false;
    playerState.seek(parseFloat((e.target as HTMLInputElement).value));
  }

  function handlePlayPrevSegment() {
    const segs = projectState.segments;
    if (!segs.length) return;
    const currId = playerState.currentPlayingSegmentId;
    const index = segs.findIndex(s => s.id === currId);
    const prevIndex = index > 0 ? index - 1 : segs.length - 1;
    const prevSeg = segs[prevIndex];
    if (prevSeg && prevSeg.audio_path) {
      playerState.playUrl(prevSeg.audio_path, prevSeg.id);
    }
  }

  function handlePlayNextSegment() {
    const segs = projectState.segments;
    if (!segs.length) return;
    const currId = playerState.currentPlayingSegmentId;
    const index = segs.findIndex(s => s.id === currId);
    const nextIndex = index >= 0 && index < segs.length - 1 ? index + 1 : 0;
    const nextSeg = segs[nextIndex];
    if (nextSeg && nextSeg.audio_path) {
      playerState.playUrl(nextSeg.audio_path, nextSeg.id);
    }
  }

  let activeSegment = $derived(
    projectState.segments.find(s => s.id === playerState.currentPlayingSegmentId)
  );
</script>

<footer class="bottom-player-bar" role="region" aria-label="Trình phát âm thanh Studio">
  <!-- Playing Meta Info -->
  <div class="player-meta">
    <div class="playing-avatar {playerState.isPlaying ? 'animated' : ''}">
      <span>{playerState.isPlaying ? '🎙️' : '🎧'}</span>
    </div>
    <div class="meta-text">
      <span class="playing-title">
        {activeSegment 
          ? `Đoạn #${activeSegment.position}: ${activeSegment.text.slice(0, 32)}...` 
          : 'Chọn phân đoạn để phát'}
      </span>
      <span class="playing-sub">
        {projectState.currentProject?.name || 'Auto TTS Desktop'} • {activeSegment?.voice || projectState.currentProject?.voice || 'Giọng đọc'}
      </span>
    </div>
  </div>

  <!-- Audio Controls (Play/Pause, Prev/Next, Seek) -->
  <div class="player-controls">
    <div class="button-row">
      <button 
        class="ctrl-btn icon-only" 
        onclick={handlePlayPrevSegment}
        title="Đoạn trước"
        aria-label="Đoạn trước"
      >
        ⏮
      </button>

      <button 
        class="ctrl-btn icon-only" 
        onclick={() => playerState.seek(Math.max(0, playerState.currentTime - 5))}
        title="Lùi 5 giây"
        aria-label="Lùi 5 giây"
      >
        ⏪
      </button>

      <button 
        class="ctrl-btn main-play-btn" 
        onclick={() => playerState.togglePlay()}
        title={playerState.isPlaying ? 'Tạm dừng (Space)' : 'Phát (Space)'}
        aria-label={playerState.isPlaying ? 'Tạm dừng' : 'Phát'}
      >
        {playerState.isPlaying ? '⏸' : '▶'}
      </button>

      <button 
        class="ctrl-btn icon-only" 
        onclick={() => playerState.seek(playerState.currentTime + 5)}
        title="Tiến 5 giây"
        aria-label="Tiến 5 giây"
      >
        ⏩
      </button>

      <button 
        class="ctrl-btn icon-only" 
        onclick={handlePlayNextSegment}
        title="Đoạn tiếp theo"
        aria-label="Đoạn tiếp theo"
      >
        ⏭
      </button>

      <button 
        class="ctrl-btn icon-only {playerState.loopSegment ? 'active' : ''}" 
        onclick={() => playerState.loopSegment = !playerState.loopSegment}
        title="Lặp lại phân đoạn này"
        aria-label="Lặp lại phân đoạn"
      >
        🔁
      </button>
    </div>

    <!-- Time Slider & Progress -->
    <div class="time-row">
      <span class="time-stamp">{formatTime(playerState.currentTime)}</span>
      <div class="range-container">
        <input 
          type="range" 
          min="0" 
          max={playerState.duration || 100} 
          value={isDragging ? localTime : playerState.currentTime}
          oninput={handleSliderInput}
          onchange={handleSliderChange}
          aria-label="Thanh thời gian phát âm thanh"
        />
        <div 
          class="range-progress" 
          style="width: {playerState.duration ? ((isDragging ? localTime : playerState.currentTime) / playerState.duration) * 100 : 0}%"
        ></div>
      </div>
      <span class="time-stamp">{formatTime(playerState.duration)}</span>
    </div>
  </div>

  <!-- Volume & Playback Rate Settings -->
  <div class="player-settings">
    <div class="setting-item volume-control">
      <span class="vol-icon">{playerState.volume === 0 ? '🔇' : '🔊'}</span>
      <input 
        type="range" 
        min="0" 
        max="1" 
        step="0.05" 
        value={playerState.volume}
        oninput={(e) => playerState.setVolume(parseFloat((e.target as HTMLInputElement).value))}
        aria-label="Âm lượng phát"
      />
    </div>

    <div class="setting-item speed-control">
      <select 
        id="speed-select" 
        value={playerState.playbackRate}
        onchange={(e) => playerState.setPlaybackRate(parseFloat((e.target as HTMLSelectElement).value))}
        aria-label="Tốc độ phát âm thanh"
      >
        <option value="0.75">0.75x</option>
        <option value="1">1.0x (Chuẩn)</option>
        <option value="1.25">1.25x</option>
        <option value="1.5">1.5x</option>
        <option value="2">2.0x</option>
      </select>
    </div>
  </div>
</footer>

<style>
  .bottom-player-bar {
    height: var(--player-height);
    background: var(--glass-bg);
    backdrop-filter: var(--glass-blur);
    border-top: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--space-6);
    gap: var(--space-4);
    z-index: 50;
    box-shadow: 0 -4px 20px rgba(0, 0, 0, 0.1);
  }

  .player-meta {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 280px;
  }

  .playing-avatar {
    width: 42px;
    height: 42px;
    border-radius: var(--radius-md);
    background: var(--color-bg-surface-raised);
    border: 1px solid var(--color-border);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    transition: transform 0.3s ease;
  }

  .playing-avatar.animated {
    animation: pulseAvatar 2s infinite ease-in-out;
    border-color: var(--color-accent);
  }

  @keyframes pulseAvatar {
    0% { transform: scale(1); }
    50% { transform: scale(1.06); }
    100% { transform: scale(1); }
  }

  .meta-text {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .playing-title {
    font-size: var(--font-size-sm);
    font-weight: 700;
    color: var(--color-text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .playing-sub {
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .player-controls {
    flex: 1;
    max-width: 580px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-1);
  }

  .button-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  .ctrl-btn {
    height: var(--target-btn-sm);
    border-radius: var(--radius-md);
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border-subtle);
    color: var(--color-text-primary);
    font-size: var(--font-size-sm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s ease;
  }

  .ctrl-btn.icon-only {
    width: 36px;
    height: 36px;
  }

  .ctrl-btn:hover {
    background: var(--color-bg-surface-hover);
    border-color: var(--color-border);
  }

  .ctrl-btn.main-play-btn {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-full);
    background: var(--gradient-primary);
    color: white;
    font-size: 1.2rem;
    border: none;
    box-shadow: 0 4px 12px rgba(37, 99, 235, 0.35);
  }
  .ctrl-btn.main-play-btn:hover {
    transform: scale(1.08);
    box-shadow: 0 6px 16px rgba(37, 99, 235, 0.45);
  }

  .ctrl-btn.active {
    background: var(--color-accent-subtle);
    color: var(--color-accent-text);
    border-color: var(--color-accent);
  }

  .time-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
  }

  .time-stamp {
    font-size: var(--font-size-xs);
    color: var(--color-text-muted);
    font-family: var(--font-mono);
    font-weight: 600;
    width: 40px;
    text-align: center;
  }

  .range-container {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }

  .range-container input[type="range"] {
    width: 100%;
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--color-bg-surface-raised);
    cursor: pointer;
    appearance: none;
  }

  .range-progress {
    position: absolute;
    left: 0;
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--gradient-primary);
    pointer-events: none;
  }

  .player-settings {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 240px;
    justify-content: flex-end;
  }

  .setting-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .setting-item.volume-control input[type="range"] {
    width: 80px;
    height: 4px;
  }

  .setting-item select {
    padding: 4px 8px;
    font-size: var(--font-size-xs);
    font-weight: 600;
    border-radius: var(--radius-sm);
    background: var(--color-bg-surface);
    border: 1px solid var(--color-border);
  }
</style>
