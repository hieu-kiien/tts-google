<script lang="ts">
  import { getErrorMessage } from "./lib/utils/errorUtils";
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import { isSegmentStatus, type ProjectRecord, type QueueSnapshot, type QueueProgressEvent } from "./lib/types/tts";
  import { toastStore } from "./lib/state/toasts.svelte";
  import { uiState } from "./lib/state/uiState.svelte";
  import { projectState } from "./lib/state/projectState.svelte";
  import { playerState } from "./lib/state/playerState.svelte";

  import { createProject, listProjects, getProjectSegments } from "./lib/api/projectClient";
  import { enqueueProject, pauseProject, getQueueSnapshot, resumeProject, cancelProject } from "./lib/api/queueClient";
  import { getApiKeyStatus, saveApiKey, testApiConnection } from "./lib/api/settingsClient";

  import Header from "./lib/components/Header.svelte";
  import ProjectSidebar from "./lib/components/ProjectSidebar.svelte";
  import EditorCentral from "./lib/components/EditorCentral.svelte";
  import InspectorRight from "./lib/components/InspectorRight.svelte";
  import BottomPlayerBar from "./lib/components/BottomPlayerBar.svelte";
  import QueueProgress from "./lib/components/QueueProgress.svelte";
  import ProjectLibrary from "./lib/components/ProjectLibrary.svelte";

  import ApiKeyModal from "./lib/components/modals/ApiKeyModal.svelte";
  import DictionaryModal from "./lib/components/modals/DictionaryModal.svelte";
  import QuickstartTemplatesModal, { type QuickstartTemplate } from "./lib/components/modals/QuickstartTemplatesModal.svelte";
  import ImportWizardModal from "./lib/components/modals/ImportWizardModal.svelte";
  import ExportModal from "./lib/components/modals/ExportModal.svelte";
  import ShortcutGuideModal from "./lib/components/modals/ShortcutGuideModal.svelte";
  import BatchProcessorModal from "./lib/components/modals/BatchProcessorModal.svelte";
  import SettingsModal from "./lib/components/SettingsModal.svelte";
  import QuotaDashboard from "./lib/components/QuotaDashboard.svelte";
  import ToastRegion from "./lib/components/ToastRegion.svelte";

  let keyConfigured = $state(false);
  let apiKeyInput = $state("");
  let rememberKey = $state(true);
  let isTestingKey = $state(false);
  let queueSnapshot = $state<QueueSnapshot | null>(null);
  let showSettingsModal = $state(false);
  let showQuotaDashboard = $state(false);

  // Global Keyboard Navigation (Section 14 & 13)
  function handleKeyDown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const isTypingOrInteracting = target && (
      target.tagName === 'INPUT' || 
      target.tagName === 'TEXTAREA' || 
      target.tagName === 'SELECT' || 
      target.tagName === 'BUTTON' || 
      target.isContentEditable ||
      target.getAttribute('role') === 'button' ||
      target.getAttribute('role') === 'slider'
    );

    if (e.ctrlKey || e.metaKey) {
      switch (e.key.toLowerCase()) {
        case 's':
          e.preventDefault();
          (async () => {
            const success = await projectState.flushPendingSaves();
            if (success) {
              toastStore.showSuccess("Đã lưu tiến độ dự án thành công.");
            } else {
              toastStore.showError("Có lỗi xảy ra khi lưu một số đoạn văn. Thay đổi chưa được lưu xuống ổ đĩa.");
            }
          })();
          break;
        case 'i':
          e.preventDefault();
          uiState.showImportWizard = true;
          break;
        case 'f':
          if (!isTypingOrInteracting) {
            e.preventDefault();
            uiState.activeView = "editor";
          }
          break;
        case 'enter':
          e.preventDefault();
          if (projectState.activeSegment) {
            const seg = projectState.activeSegment;
            if (seg.audio_path && seg.status !== 'stale') {
              playerState.playUrl(seg.audio_path, seg.id);
            } else {
              toastStore.showInfo(`Đoạn #${seg.position} chưa có audio. Nhấn nút Nghe Thử để phát.`);
            }
          }
          break;
        case '.':
          e.preventDefault();
          playerState.stop();
          toastStore.showInfo("Đã dừng phát âm thanh.");
          break;
        case 'j':
          e.preventDefault();
          toastStore.showInfo("Đã bật điều hướng hàng đợi.");
          break;
      }
    } else if (e.key === ' ' && !isTypingOrInteracting) {
      e.preventDefault();
      playerState.togglePlay();
    }
  }

  async function checkKeyStatus() {
    try {
      const res = await getApiKeyStatus();
      keyConfigured = res.configured;
    } catch (err: unknown) {
      console.warn("Key status check:", getErrorMessage(err));
    }
  }

  async function handleSaveKey() {
    if (!apiKeyInput.trim()) {
      toastStore.showError("Vui lòng nhập Gemini API Key");
      return;
    }
    try {
      const res = await saveApiKey(apiKeyInput, rememberKey);
      keyConfigured = res.configured;
      apiKeyInput = "";
      uiState.showApiKeyModal = false;
      toastStore.showSuccess("Đã lưu API Key thành công!");
    } catch (err: unknown) {
      toastStore.showError("Lỗi lưu Key: " + getErrorMessage(err));
    }
  }

  async function handleTestConnection() {
    try {
      isTestingKey = true;
      const res = await testApiConnection(apiKeyInput.trim() ? apiKeyInput : null);
      toastStore.showSuccess(res);
    } catch (err: unknown) {
      toastStore.showError(getErrorMessage(err));
    } finally {
      isTestingKey = false;
    }
  }


  async function handleStartQueue() {
    if (!keyConfigured) {
      toastStore.showError("Vui lòng cấu hình Gemini API Key trước khi bắt đầu tạo audio!");
      uiState.showApiKeyModal = true;
      return;
    }
    
    if (!projectState.currentProject) {
      toastStore.showError("Chưa có dự án nào được chọn.");
      return;
    }

    await projectState.flushPendingSaves();

    try {
      if (projectState.currentProject?.id) {
        toastStore.showInfo("Đang khởi động tiến trình tạo giọng đọc...");
        const snap = await enqueueProject(projectState.currentProject.id);
        queueSnapshot = snap;
        const segs = await getProjectSegments(projectState.currentProject.id);
        projectState.segments = segs;
        toastStore.showSuccess("Đã khởi động trình tạo giọng đọc thành công!");
      } else {
        toastStore.showInfo("Đang tạo dự án mới...");
        const proj = await createProject({
          name: projectState.currentProject.name,
          sourceText: projectState.currentProject.source_text,
          voice: projectState.currentProject.voice || "Kore",
          preset: projectState.currentProject.preset || "Tự nhiên",
        });
        
        projectState.currentProject = proj;
        const segs = await getProjectSegments(proj.id);
        projectState.segments = segs;

        toastStore.showInfo("Đang khởi động tiến trình tạo giọng đọc...");
        const snap = await enqueueProject(proj.id);
        queueSnapshot = snap;
        toastStore.showSuccess("Đã khởi động trình tạo giọng đọc thành công!");
      }
    } catch (err: unknown) {
      toastStore.showError("Lỗi khởi động: " + getErrorMessage(err));
    }
  }

  async function handlePauseQueue() {
    if (!projectState.currentProject) return;
    try {
      await pauseProject(projectState.currentProject.id);
      toastStore.showInfo("Đã tạm dừng hàng đợi thành công");
    } catch (err: unknown) {
      toastStore.showError("Lỗi tạm dừng: " + getErrorMessage(err));
    }
  }

  function handleResize() {
    if (typeof window !== "undefined") {
      const isNarrow = window.innerWidth < 1024;
      uiState.isNarrowWindow = isNarrow;
      if (isNarrow) {
        uiState.showSidebar = false;
        uiState.showInspector = false;
      }
    }
  }

  onMount(() => {
    checkKeyStatus();
    handleResize();

    let isMounted = true;
    const unlistenFns: UnlistenFn[] = [];

    const handleQueueProgress = (event: { payload: QueueProgressEvent }) => {
      if (!projectState.currentProject || event.payload.project_id !== projectState.currentProject.id) {
        return;
      }
      const progress = event.payload;
      if (progress.segment_id && projectState.segments) {
        const segIdx = projectState.segments.findIndex(s => s.id === progress.segment_id);
        if (segIdx !== -1) {
          if (isSegmentStatus(progress.status)) {
            projectState.segments[segIdx].status = progress.status;
          }
          if (progress.error_message) {
            projectState.segments[segIdx].error_message = progress.error_message;
          }
        }
      }
      if (queueSnapshot) {
        queueSnapshot = {
          ...queueSnapshot,
          total_segments: progress.total_segments,
          completed_segments: progress.completed_segments,
        };
      }
    };

    const handleQueueSnapshot = (event: { payload: QueueSnapshot }) => {
      if (projectState.currentProject && event.payload.project_id === projectState.currentProject.id) {
        queueSnapshot = event.payload;
      }
    };

    const registerListener = async (
      registration: Promise<UnlistenFn>
    ): Promise<void> => {
      try {
        const unlisten = await registration;
        if (isMounted) {
          unlistenFns.push(unlisten);
        } else {
          unlisten();
        }
      } catch (err) {
        console.warn("Không thể đăng ký queue listener:", err);
      }
    };

    void registerListener(
      listen<QueueProgressEvent>("queue-progress", handleQueueProgress)
    );
    void registerListener(
      listen<QueueSnapshot>("queue-snapshot", handleQueueSnapshot)
    );

    // Load projects from database on startup
    (async () => {
      try {
        const dbProjects = await listProjects();
        if (dbProjects.length > 0) {
          projectState.projects = dbProjects;
          projectState.currentProject = dbProjects[0];
          try {
            const segs = await getProjectSegments(dbProjects[0].id);
            projectState.segments = segs;
          } catch {
            projectState.segments = [];
          }
        } else {
          uiState.activeView = "library";
        }
      } catch (err) {
        console.warn("Lỗi load projects từ DB:", err);
        uiState.activeView = "library";
      }
    })();

    // === CLOSE HANDLER: flush pending edits before window closes ===
    const appWindow = getCurrentWindow();
    let unlistenClose: (() => void) | null = null;
    appWindow.onCloseRequested(async (event) => {
      if (!projectState.hasPendingSaves) {
        return; // No pending saves, allow close
      }

      event.preventDefault();

      try {
        const success = await projectState.flushPendingSaves();
        if (success) {
          await appWindow.destroy();
        } else {
          // Some saves failed — ask user
          const forceClose = confirm(
            'Một số đoạn chưa lưu được. Bạn có muốn đóng ứng dụng không?\n\n' +
            '• OK = Đóng không lưu\n' +
            '• Cancel = Quay lại ứng dụng'
          );
          if (forceClose) {
            await appWindow.destroy();
          }
        }
      } catch (error) {
        console.error('Error flushing saves on close:', error);
        const forceClose = confirm(
          'Lỗi khi lưu dữ liệu: ' + getErrorMessage(error) + '\n\n' +
          'Bạn có muốn đóng ứng dụng không?'
        );
        if (forceClose) {
          await appWindow.destroy();
        }
      }
    }).then(fn => { unlistenClose = fn; });

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("resize", handleResize);

    return () => {
      isMounted = false;
      if (unlistenClose) unlistenClose();
      for (const unlisten of unlistenFns.splice(0)) {
        try {
          unlisten();
        } catch (err) {
          console.warn("Không thể hủy queue listener:", err);
        }
      }
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("resize", handleResize);
    };
  });
</script>

<div class="app-shell" data-theme={uiState.theme}>
  <!-- Zone A: Top Header & Main Toolbar -->
  <Header 
    {keyConfigured}
    onOpenApiKeyModal={() => uiState.showApiKeyModal = true}
    onOpenDictionary={() => uiState.showDictionaryModal = true}
    onStartQueue={handleStartQueue}
    onPauseQueue={handlePauseQueue}
    onOpenQuotaDashboard={() => showQuotaDashboard = true}
    onOpenSettings={() => showSettingsModal = true}
  />

  <!-- Main 3-Column Layout (Zones B, C, D) -->
  <div class="app-body">
    {#if uiState.activeView === 'library'}
      <ProjectLibrary />
    {:else if uiState.activeView === 'export'}
      <ExportModal />
    {:else}
      <!-- Zone B: Left Sidebar (Chapters, Segments, Filters) -->
      {#if uiState.showSidebar}
        <ProjectSidebar />
      {/if}

      <!-- Zone C: Central Main Editor -->
      <EditorCentral />

      <!-- Zone D: Right Inspector (Voice Picker, Style, Audio Result Review) -->
      {#if uiState.showInspector}
        <InspectorRight />
      {/if}
    {/if}
  </div>

  <!-- Zone E: Queue Progress Bar & Fixed Bottom Player -->
  <div class="app-bottom-panel">
    <QueueProgress 
      snapshot={queueSnapshot}
      onEnqueue={handleStartQueue}
      onPause={handlePauseQueue}
      onResume={async () => {
        if (projectState.currentProject?.id) {
          try {
            await resumeProject(projectState.currentProject.id);
            toastStore.showInfo("Đã tiếp tục hàng đợi");
          } catch (err: unknown) {
            toastStore.showError("Lỗi tiếp tục: " + getErrorMessage(err));
          }
        }
      }}
      onCancel={async () => {
        if (projectState.currentProject?.id) {
          try {
            await cancelProject(projectState.currentProject.id);
            toastStore.showInfo("Đã hủy hàng đợi");
          } catch (err: unknown) {
            toastStore.showError("Lỗi hủy: " + getErrorMessage(err));
          }
        }
      }}
    />
    <BottomPlayerBar />
  </div>

  <!-- Modals & Overlays -->
  <ApiKeyModal 
    show={uiState.showApiKeyModal}
    isTesting={isTestingKey}
    bind:apiKeyInput
    bind:rememberKey
    onClose={() => uiState.showApiKeyModal = false}
    onSave={handleSaveKey}
    onTest={handleTestConnection}
  />

  <DictionaryModal 
    show={uiState.showDictionaryModal}
    rules={projectState.dictionaryRules}
    onClose={() => uiState.showDictionaryModal = false}
    onAddRule={(find: string, replace: string) => projectState.addDictionaryRule(find, replace)}
    onRemoveRule={(id: string) => projectState.dictionaryRules = projectState.dictionaryRules.filter(r => r.id !== id)}
    onLoadDefault={() => projectState.loadDefaultRules()}
  />

  <QuickstartTemplatesModal 
    show={uiState.showTemplatesModal}
    onClose={() => uiState.showTemplatesModal = false}
    onSelectTemplate={async (tpl: QuickstartTemplate) => {
      try {
        const newProj = await createProject({
          name: `Dự án mẫu: ${tpl.title}`,
          sourceText: tpl.sampleText,
          voice: tpl.defaultVoice,
          preset: tpl.preset,
        });
        projectState.projects = [newProj, ...projectState.projects];
        projectState.currentProject = newProj;
        const segs = await getProjectSegments(newProj.id);
        projectState.segments = segs;
        uiState.activeView = "editor";
        toastStore.showSuccess(`Đã tạo dự án mẫu "${tpl.title}" thành công!`);
      } catch (err: unknown) {
        toastStore.showError(`Lỗi tạo dự án mẫu: ${getErrorMessage(err)}`);
      }
      uiState.showTemplatesModal = false;
    }}
  />

  <ImportWizardModal />
  <ShortcutGuideModal />
  {#if uiState.showBatchProcessorModal}
    <BatchProcessorModal />
  {/if}

  <SettingsModal isOpen={showSettingsModal} onClose={() => showSettingsModal = false} />
  <QuotaDashboard isOpen={showQuotaDashboard} onClose={() => showQuotaDashboard = false} />

  <ToastRegion />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background-color: var(--color-bg-app);
    color: var(--color-text-primary);
  }

  .app-body {
    flex: 1;
    display: flex;
    overflow: hidden;
    position: relative;
  }

  .app-bottom-panel {
    display: flex;
    flex-direction: column;
    z-index: 50;
  }
</style>
