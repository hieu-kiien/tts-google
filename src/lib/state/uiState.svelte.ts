// UI State management using Svelte 5 runes

export type ActiveView = "library" | "editor" | "export";
export type InspectorTab = "segment" | "voice" | "style" | "review";

class UiState {
  theme = $state<"light" | "dark">(
    (typeof window !== 'undefined' && localStorage.getItem('tts-theme') as "light" | "dark") || "light"
  );
  activeView = $state<ActiveView>("editor");
  showSidebar = $state<boolean>(true);
  showInspector = $state<boolean>(true);
  inspectorTab = $state<InspectorTab>("voice");
  
  // Active Modals
  showApiKeyModal = $state<boolean>(false);
  showDictionaryModal = $state<boolean>(false);
  showImportWizard = $state<boolean>(false);
  showShortcutGuide = $state<boolean>(false);
  showTemplatesModal = $state<boolean>(false);
  showBatchProcessorModal = $state<boolean>(false);

  // Editor State
  selectedSegmentIds = $state<string[]>([]);
  segmentSearchQuery = $state<string>("");
  statusFilter = $state<string>("all");
  autoScrollToPlaying = $state<boolean>(true);

  // Responsive Drawer (for mobile / narrow screens < 1024px)
  isNarrowWindow = $state<boolean>(false);

  toggleTheme() {
    this.theme = this.theme === "light" ? "dark" : "light";
    document.documentElement.setAttribute("data-theme", this.theme);
    localStorage.setItem('tts-theme', this.theme);
  }

  setTheme(newTheme: "light" | "dark") {
    this.theme = newTheme;
    document.documentElement.setAttribute("data-theme", this.theme);
    localStorage.setItem('tts-theme', this.theme);
  }
}

export const uiState = new UiState();
