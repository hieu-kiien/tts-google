import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { projectState } from "./projectState.svelte";
import { toastStore } from "./toasts.svelte";
import { getErrorMessage } from "../utils/errorUtils";

// Player & Audio Runtime State using Svelte 5 runes with Instant Streaming & Autoplay support

class PlayerState {
  isPlaying = $state<boolean>(false);
  currentTime = $state<number>(0);
  duration = $state<number>(0);
  volume = $state<number>(1.0);
  playbackRate = $state<number>(1.0);
  
  currentPlayingSegmentId = $state<string | null>(null);
  currentPlayingAudioUrl = $state<string | null>(null);
  loopSegment = $state<boolean>(false);

  private audioElement: HTMLAudioElement | null = null;

  initAudio() {
    if (typeof window !== "undefined" && !this.audioElement) {
      this.audioElement = new Audio();
      this.audioElement.onplay = () => { 
        this.isPlaying = true; 
        if (this.audioElement) this.audioElement.playbackRate = this.playbackRate;
      };
      this.audioElement.oncanplay = () => {
        if (this.audioElement) this.audioElement.playbackRate = this.playbackRate;
      };
      this.audioElement.onpause = () => { this.isPlaying = false; };
      this.audioElement.onended = () => {
        this.isPlaying = false;
        if (this.loopSegment && this.currentPlayingAudioUrl) {
          this.playUrl(this.currentPlayingAudioUrl, this.currentPlayingSegmentId);
        } else if (this.currentPlayingSegmentId && projectState.segments.length > 0) {
          const currentIdx = projectState.segments.findIndex(s => s.id === this.currentPlayingSegmentId);
          if (currentIdx !== -1) {
            // Find the *next* segment that actually has audio
            let nextSegIdx = currentIdx + 1;
            while (nextSegIdx < projectState.segments.length) {
              if (projectState.segments[nextSegIdx].audio_path) {
                break;
              }
              nextSegIdx++;
            }
            
            if (nextSegIdx < projectState.segments.length) {
              const nextSeg = projectState.segments[nextSegIdx];
              projectState.activeSegmentId = nextSeg.id;
              // Smooth scroll to next segment card
              if (typeof document !== 'undefined') {
                const el = document.getElementById(`seg_card_${nextSeg.id}`);
                if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
              }
              this.playUrl(nextSeg.audio_path as string, nextSeg.id);
            }
          }
        }
      };
      this.audioElement.ontimeupdate = () => {
        if (this.audioElement) {
          this.currentTime = this.audioElement.currentTime;
          this.duration = this.audioElement.duration || 0;
        }
      };
    }
  }

  async playUrl(url: string, segmentId: string | null = null) {
    if (!url) return;
    this.initAudio();
    if (!this.audioElement) return;

    let targetUrl = url;
    if (!targetUrl.startsWith("data:") && !targetUrl.startsWith("http:") && !targetUrl.startsWith("https:") && !targetUrl.startsWith("asset:")) {
      try {
        targetUrl = convertFileSrc(url);
      } catch {
        try {
          targetUrl = await invoke<string>("read_audio_data_url", { path: url, filePath: url });
        } catch (e) {
          console.warn("Failed to convert audio path to data url:", e);
        }
      }
    }

    const cacheBuster = targetUrl.startsWith("data:") ? "" : `?t=${Date.now()}`;
    
    if (this.currentPlayingAudioUrl !== targetUrl) {
      // Append cache buster so browser reloads audio if the file on disk was overwritten
      this.audioElement.src = targetUrl + cacheBuster;
      this.currentPlayingAudioUrl = targetUrl;
      this.currentPlayingSegmentId = segmentId;
      this.audioElement.currentTime = 0;
    } else {
      // Still append cache buster in case user regenerated same segment
      this.audioElement.src = targetUrl + cacheBuster;
      
      // Re-playing same audio: if finished or paused, reset currentTime to start
      if (this.audioElement.ended || this.audioElement.currentTime >= this.audioElement.duration) {
        this.audioElement.currentTime = 0;
      }
      this.currentPlayingSegmentId = segmentId;
    }

    this.audioElement.volume = this.volume;
    this.audioElement.playbackRate = this.playbackRate;
    try {
      await this.audioElement.play();
      if (this.audioElement) this.audioElement.playbackRate = this.playbackRate;
    } catch (e) {
      console.error("Playback error:", e);
    }
  }

  pause() {
    if (this.audioElement) {
      this.audioElement.pause();
    }
  }

  async togglePlay() {
    if (!this.audioElement || !this.currentPlayingAudioUrl) return;
    if (this.isPlaying) {
      this.pause();
    } else {
      try {
        await this.audioElement.play();
      } catch (err: unknown) {
        this.isPlaying = false;
        toastStore.showError("Không thể phát âm thanh: " + getErrorMessage(err));
      }
    }
  }

  stop() {
    if (this.audioElement) {
      this.audioElement.pause();
      this.audioElement.currentTime = 0;
      this.isPlaying = false;
    }
  }

  seek(seconds: number) {
    if (this.audioElement) {
      this.audioElement.currentTime = seconds;
      this.currentTime = seconds;
    }
  }

  setVolume(vol: number) {
    this.volume = vol;
    if (this.audioElement) {
      this.audioElement.volume = vol;
    }
  }

  setPlaybackRate(rate: number) {
    this.playbackRate = rate;
    if (this.audioElement) {
      this.audioElement.playbackRate = rate;
    }
  }

  dispose() {
    if (this.audioElement) {
      this.audioElement.pause();
      this.audioElement.onplay = null;
      this.audioElement.oncanplay = null;
      this.audioElement.onpause = null;
      this.audioElement.onended = null;
      this.audioElement.ontimeupdate = null;
      this.audioElement = null;
    }
    this.isPlaying = false;
    this.currentTime = 0;
    this.duration = 0;
    this.currentPlayingSegmentId = null;
    this.currentPlayingAudioUrl = null;
  }

  reset() {
    this.dispose();
  }
}

export const playerState = new PlayerState();
