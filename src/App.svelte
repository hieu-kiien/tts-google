<script lang="ts">
  import { getErrorMessage } from "./lib/utils/errorUtils";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  import type { ProjectRecord, QueueSnapshot, QueueProgressEvent } from "./lib/types/tts";
  import { toastStore } from "./lib/state/toasts.svelte";
  import { uiState } from "./lib/state/uiState.svelte";
  import { projectState } from "./lib/state/projectState.svelte";
  import { playerState } from "./lib/state/playerState.svelte";

  import { createProject, listProjects, getProjectSegments } from "./lib/api/projectClient";
  import { enqueueProject, pauseProject, getQueueSnapshot, resumeProject, cancelProject } from "./lib/api/queueClient";

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
  import ToastRegion from "./lib/components/ToastRegion.svelte";

  let keyConfigured = $state(false);
  let apiKeyInput = $state("");
  let rememberKey = $state(true);
  let isTestingKey = $state(false);
  let queueSnapshot = $state<QueueSnapshot | null>(null);

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
          projectState.flushPendingSaves();
          toastStore.showSuccess("Đã lưu tiến độ dự án thành công.");
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
      const res = await invoke<{ configured: boolean }>("get_api_key_status");
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
      const res = await invoke<{ configured: boolean }>("save_api_key", {
        key: apiKeyInput,
        remember: rememberKey,
      });
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
      const res = await invoke<string>("test_api_connection", {
        testKey: apiKeyInput.trim() ? apiKeyInput : null,
      });
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

  onMount(() => {
    checkKeyStatus();

    let unlisten: UnlistenFn | undefined;

    listen<QueueSnapshot>("queue-progress", (event) => {
      queueSnapshot = event.payload;
    }).then(fn => {
      unlisten = fn;
    }).catch(err => {
      console.warn("Lỗi đăng ký sự kiện queue-progress:", err);
    });

    // Load projects from database on startup
    (async () => {
      try {
        const dbProjects = await invoke<ProjectRecord[]>("list_projects");
        if (dbProjects.length > 0) {
          projectState.projects = dbProjects;
          projectState.currentProject = dbProjects[0];
          // Load segments for the first project
          try {
            const segs = await invoke<import("./lib/types/tts").SegmentRecord[]>("get_project_segments", { projectId: dbProjects[0].id });
            projectState.segments = segs;
          } catch {
            projectState.segments = [];
          }
        } else {
          // No projects in DB — show library for user to create one
          uiState.activeView = "library";
        }
      } catch (err) {
        console.warn("Lỗi load projects từ DB:", err);
        uiState.activeView = "library";
      }
    })();

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      if (unlisten) unlisten();
      window.removeEventListener("keydown", handleKeyDown);
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
