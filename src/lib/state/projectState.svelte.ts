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

export type PendingSegmentSave = {
  segmentId: string;
  projectId: string;
  text: string;
  clientRevision: number;
};

class ProjectState {
  projects = $state<ProjectRecord[]>([]);
  currentProject = $state<ProjectRecord | null>(null);
  segments = $state<SegmentRecord[]>([]);
  dictionaryRules = $state<PronunciationRule[]>([...defaultStandardRules]);
  
  activeSegmentId = $state<string | null>(null);

  private _debounceTimers: Map<string, ReturnType<typeof setTimeout>> = new Map();
  private _pendingSaveMap: Map<string, PendingSegmentSave> = new Map();
  private _clientRevisions: Map<string, number> = new Map();
  private _compiledRules: { regex: RegExp; replace: string }[] | null = null;
  private _rulesHash: string = '';

  // Helper to compute spoken text with current dictionary rules
  computeSpokenText(originalText: string): string {
    const currentHash = this.dictionaryRules.map(r => r.find + r.replace).join('|');
    if (currentHash !== this._rulesHash) {
      this._rulesHash = currentHash;
      this._compiledRules = this.dictionaryRules
        .filter(r => r.find.trim())
        .map(r => ({
          regex: new RegExp(r.find.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi'),
          replace: r.replace,
        }));
    }

    let result = originalText;
    for (const rule of this._compiledRules!) {
      result = result.replace(rule.regex, rule.replace);
    }
    // Number and currency expansions - MUST process larger patterns first
    result = result.replace(/(\d+)\.000\.000\s*đ/gi, '$1 triệu đồng');
    result = result.replace(/(\d+)\.000\s*đ/gi, '$1 nghìn đồng');
    return result;
  }

  get hasPendingSaves(): boolean {
    return this._pendingSaveMap.size > 0 || this._debounceTimers.size > 0;
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

  async setSegmentReviewStatus(segmentId: string, status: ReviewStatus, issue: SegmentIssue | null = null) {
    const seg = this.segments.find(s => s.id === segmentId);
    if (seg) {
      seg.review_status = status;
      seg.reported_issue = issue;
      seg.reviewed_output_fingerprint = status === "approved" ? (seg.output_fingerprint || null) : null;
      
      try {
        await invoke("update_segment_review_status", {
          segmentId: seg.id,
          reviewStatus: status,
          reviewedOutputFingerprint: seg.reviewed_output_fingerprint,
        });
      } catch (err: unknown) {
        console.warn("Lỗi update_segment_review_status IPC:", getErrorMessage(err));
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
    const spokenTimerKey = `spoken_${segmentId}`;
    const existingSpokenTimer = this._debounceTimers.get(spokenTimerKey);
    if (existingSpokenTimer) clearTimeout(existingSpokenTimer);

    this._debounceTimers.set(spokenTimerKey, setTimeout(() => {
      seg.spoken_text = this.computeSpokenText(newText);
      this._debounceTimers.delete(spokenTimerKey);
    }, 300));

    if (this.currentProject?.id) {
      const projectId = this.currentProject.id;
      const revision = (this._clientRevisions.get(segmentId) || 0) + 1;
      this._clientRevisions.set(segmentId, revision);

      this._pendingSaveMap.set(segmentId, {
        segmentId,
        projectId,
        text: newText,
        clientRevision: revision,
      });

      const ipcTimerKey = `ipc_${segmentId}`;
      const existingIpcTimer = this._debounceTimers.get(ipcTimerKey);
      if (existingIpcTimer) clearTimeout(existingIpcTimer);

      this._debounceTimers.set(ipcTimerKey, setTimeout(async () => {
        this._debounceTimers.delete(ipcTimerKey);
        await this.saveSingleSegment(segmentId, projectId, newText, revision);
      }, 500));
    }
  }

  private async saveSingleSegment(segmentId: string, projectId: string, text: string, revision: number): Promise<boolean> {
    try {
      await invoke("update_segment_text", {
        projectId,
        segmentId,
        text,
      });
      const pending = this._pendingSaveMap.get(segmentId);
      if (pending && pending.clientRevision === revision) {
        this._pendingSaveMap.delete(segmentId);
      }
      return true;
    } catch (err: unknown) {
      console.warn(`Lỗi lưu DB cho segment ${segmentId}:`, getErrorMessage(err));
      return false;
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

  async flushPendingSaves(): Promise<boolean> {
    // === ATOMIC SWAP ===
    // Snapshot the current pending state, then replace with fresh maps.
    // Any edits arriving AFTER this point go into the new maps and won't be lost.
    const timersSnapshot = this._debounceTimers;
    const pendingSnapshot = this._pendingSaveMap;
    this._debounceTimers = new Map();
    this._pendingSaveMap = new Map();

    // Cancel all timers from the snapshot
    for (const timer of timersSnapshot.values()) {
      clearTimeout(timer);
    }

    if (pendingSnapshot.size === 0) {
      return true;
    }

    // Flush snapshot — use allSettled so one failure doesn't block others
    const pendingEntries = Array.from(pendingSnapshot.values());
    const results = await Promise.allSettled(
      pendingEntries.map(entry =>
        invoke("update_segment_text", {
          projectId: entry.projectId,
          segmentId: entry.segmentId,
          text: entry.text,
        })
      )
    );

    // Re-queue failed saves ONLY if no newer edit exists in the current map
    let allSucceeded = true;
    for (let i = 0; i < results.length; i++) {
      if (results[i].status === "rejected") {
        allSucceeded = false;
        const entry = pendingEntries[i];
        if (!this._pendingSaveMap.has(entry.segmentId)) {
          // No newer edit → re-queue for retry
          this._pendingSaveMap.set(entry.segmentId, entry);
          console.warn(`Failed to save segment ${entry.segmentId}, will retry`);
        }
        // If newer edit exists → skip (the new edit's debounce will handle it)
      }
    }

    return allSucceeded && this._pendingSaveMap.size === 0;
  }

  addDictionaryRule(find: string, replace: string) {
    const newRule: PronunciationRule = {
      id: Date.now().toString(),
      find: find.trim(),
      replace: replace.trim()
    };
    this.dictionaryRules = [...this.dictionaryRules, newRule];
    this._rulesHash = '';
  }

  loadDefaultRules() {
    this.dictionaryRules = [...defaultStandardRules];
    this._rulesHash = '';
  }
}

export const projectState = new ProjectState();
