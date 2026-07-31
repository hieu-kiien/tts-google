import type { ProjectRecord, SegmentRecord, PronunciationRule, ReviewStatus, SegmentIssue } from "../types/tts";
import { invoke } from "@tauri-apps/api/core";
import { getErrorMessage } from "../utils/errorUtils";

// Default Vietnamese Pronunciation Normalization Rules
export const defaultStandardRules: PronunciationRule[] = [
  { id: "1", find: "USD", replace: "đô la Mỹ" },
  { id: "2", find: "EUR", replace: "ơ-rô" },
  { id: "3", find: "VND", replace: "Việt Nam đồng" },
  { id: "4", find: "VNĐ", replace: "Việt Nam đồng" },
  { id: "5", find: "TP.HCM", replace: "Thành phố Hồ Chí Minh" },
  { id: "6", find: "TPHCM", replace: "Thành phố Hồ Chí Minh" },
  { id: "7", find: "HN", replace: "Hà Nội" },
  { id: "8", find: "NXB", replace: "Nhà xuất bản" },
  { id: "9", find: "UBND", replace: "Ủy ban nhân dân" },
  { id: "10", find: "THPT", replace: "Trung học phổ thông" },
  { id: "11", find: "PGS.TS", replace: "Phó Giáo sư Tiến sĩ" },
  { id: "12", find: "GS.TS", replace: "Giáo sư Tiến sĩ" },
  { id: "13", find: "Dr.", replace: "Bác sĩ" },
  { id: "14", find: "km", replace: "ki-lô-mét" },
  { id: "15", find: "kg", replace: "ki-lô-gam" },
  { id: "16", find: "m²", replace: "mét vuông" },
  { id: "17", find: "GB", replace: "gi-ga-bay" },
  { id: "18", find: "AI", replace: "Trí tuệ nhân tạo E-Ai" },
  { id: "19", find: "Google", replace: "Gú-gồ" },
  { id: "20", find: "TTS", replace: "Ti-ti-ép" },
  { id: "21", find: "Website", replace: "Trang web" },
  { id: "22", find: "App", replace: "Ứng dụng" },
];

class ProjectState {
  projects = $state<ProjectRecord[]>([]);
  currentProject = $state<ProjectRecord | null>(null);
  segments = $state<SegmentRecord[]>([]);
  dictionaryRules = $state<PronunciationRule[]>([...defaultStandardRules]);
  
  activeSegmentId = $state<string | null>(null);

  private _debounceTimers: Map<string, ReturnType<typeof setTimeout>> = new Map();

  // Helper to compute spoken text with current dictionary rules
  computeSpokenText(originalText: string): string {
    let result = originalText;
    for (const rule of this.dictionaryRules) {
      if (rule.find.trim()) {
        // Use escaped literal match instead of \b for Unicode compatibility
        // Simple literal match - more reliable than Unicode lookbehind
        const escaped = rule.find.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        const regex = new RegExp(escaped, 'gi');
        result = result.replace(regex, rule.replace);
      }
    }
    // Number and currency expansions - MUST process larger patterns first
    result = result.replace(/(\d+)\.000\.000\s*đ/gi, '$1 triệu đồng');
    result = result.replace(/(\d+)\.000\s*đ/gi, '$1 nghìn đồng');
    return result;
  }

  get hasPendingSaves(): boolean {
    return this._debounceTimers.size > 0;
  }

  get activeSegment(): SegmentRecord | null {
    if (!this.activeSegmentId) return this.segments[0] || null;
    return this.segments.find(s => s.id === this.activeSegmentId) || null;
  }

  toggleSegmentLock(segmentId: string) {
    const seg = this.segments.find(s => s.id === segmentId);
    if (seg) {
      seg.is_locked = !seg.is_locked;
    }
  }

  toggleSegmentSkip(segmentId: string) {
    const seg = this.segments.find(s => s.id === segmentId);
    if (seg) {
      seg.is_skipped = !seg.is_skipped;
    }
  }

  setSegmentReviewStatus(segmentId: string, status: ReviewStatus, issue: SegmentIssue | null = null) {
    const seg = this.segments.find(s => s.id === segmentId);
    if (seg) {
      seg.review_status = status;
      seg.reported_issue = issue;
      if (status === "approved") {
        seg.status = "approved";
      }
    }
  }

  approveSegment(segmentId: string) {
    this.setSegmentReviewStatus(segmentId, "approved", null);
  }

  async updateSegmentText(segmentId: string, newText: string) {
    const seg = this.segments.find(s => s.id === segmentId);
    if (!seg) return;

    if (seg.status === "success" || seg.status === "approved") {
      seg.status = "stale";
    }

    seg.text = newText;

    // Debounce computeSpokenText: only recompute after 300ms of no typing
    const existingTimer = this._debounceTimers.get(segmentId);
    if (existingTimer) clearTimeout(existingTimer);

    this._debounceTimers.set(segmentId, setTimeout(() => {
      seg.spoken_text = this.computeSpokenText(newText);
      this._debounceTimers.delete(segmentId);
    }, 300));

    // Debounce IPC call to backend too
    if (this.currentProject?.id) {
      const projectId = this.currentProject.id; // Capture NOW to avoid race
      const ipcTimerKey = `ipc_${segmentId}`;
      const existingIpcTimer = this._debounceTimers.get(ipcTimerKey);
      if (existingIpcTimer) clearTimeout(existingIpcTimer);

      this._debounceTimers.set(ipcTimerKey, setTimeout(async () => {
        try {
          await invoke("update_segment_text", {
            projectId: projectId, // Use captured value
            segmentId: seg.id,
            text: newText
          });
        } catch (err: unknown) {
          console.warn("Lỗi update_segment_text IPC:", getErrorMessage(err));
        }
        this._debounceTimers.delete(ipcTimerKey);
      }, 500));
    }
  }

  async replaceAllText(query: string, replacement: string): Promise<number> {
    if (!query || !this.segments.length) return 0;
    let occurrencesCount = 0;
    const modifiedSegments: SegmentRecord[] = [];

    this.segments = this.segments.map(seg => {
      if (seg.text.includes(query)) {
        const matches = seg.text.split(query).length - 1;
        occurrencesCount += matches;
        const newText = seg.text.replaceAll(query, replacement);
        const updatedSeg: SegmentRecord = {
          ...seg,
          text: newText,
          spoken_text: this.computeSpokenText(newText),
          status: seg.status === 'success' || seg.status === 'approved' ? 'stale' : seg.status
        };
        modifiedSegments.push(updatedSeg);
        return updatedSeg;
      }
      return seg;
    });

    if (modifiedSegments.length > 0 && this.currentProject?.id) {
      const projectId = this.currentProject.id;
      for (const seg of modifiedSegments) {
        try {
          await invoke("update_segment_text", {
            projectId: projectId,
            segmentId: seg.id,
            text: seg.text
          });
        } catch (err: unknown) {
          console.warn(`Lỗi lưu DB cho segment #${seg.position}:`, getErrorMessage(err));
        }
      }
    }

    return occurrencesCount;
  }

  flushPendingSaves() {
    for (const [key, timer] of this._debounceTimers.entries()) {
      clearTimeout(timer);
    }
    this._debounceTimers.clear();
  }

  addDictionaryRule(find: string, replace: string) {
    const newRule: PronunciationRule = {
      id: Date.now().toString(),
      find: find.trim(),
      replace: replace.trim()
    };
    this.dictionaryRules = [...this.dictionaryRules, newRule];
  }

  loadDefaultRules() {
    this.dictionaryRules = [...defaultStandardRules];
  }
}

export const projectState = new ProjectState();
