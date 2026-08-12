import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useRef, useState } from "react";
import { ModelsModal } from "./components/ModelsModal";
import { SpeakerModelsModal } from "./components/SpeakerModelsModal";
import {
  cancelTranscription,
  deleteModel,
  deleteSpeakerModel,
  downloadModel,
  downloadSpeakerModel,
  errorMessage,
  exportTranscript,
  listModels,
  listSpeakerModels,
  selectCustomModel,
  selectCustomSpeakerModel,
  transcribeFile
} from "./lib/tauri";
import { copyFailureFeedback, copySuccessFeedback } from "./lib/feedback";
import { filterLiveTranscriptSegments, formatTranscriptSegments } from "./lib/transcriptFilter";
import type {
  BenchmarkPhase,
  DownloadProgress,
  ModelInfo,
  SpeakerModelInfo,
  TranscriptionBenchmark,
  TranscriptSegment,
  TranscriptionError,
  TranscriptionProgress,
  TranscriptionSegmentEvent
} from "./types/localscribe";

type UiState = "loading" | "no-model" | "ready" | "downloading" | "transcribing" | "completed" | "error";
type InterfaceLanguage = "en" | "ru";

interface DragState {
  isDragging: boolean;
  dragCounter: number;
}

const languages = [
  { value: "auto", label: "Auto" },
  { value: "multi", label: "Multilingual Quality" },
  { value: "multi-fast", label: "Multilingual Fast" },
  { value: "ru", label: "Russian" },
  { value: "en", label: "English" },
  { value: "de", label: "German" },
  { value: "fr", label: "French" },
  { value: "es", label: "Spanish" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
  { value: "zh", label: "Chinese" },
  { value: "ja", label: "Japanese" }
];

const translations: Record<InterfaceLanguage, Record<string, string>> = {
  en: {
    appName: "LocalScribe",
    appDescription: "Local audio and video transcription. No cloud upload.",
    stateLoading: "Loading",
    stateNoModel: "No model",
    stateReady: "Ready",
    stateDownloading: "Downloading",
    stateTranscribing: "Transcribing",
    stateCompleted: "Completed",
    stateError: "Error",
    dropZoneDefault: "Drop audio/video here",
    dropZoneDragging: "Drop file here",
    dropZoneFormats: "aac, mp3, wav, m4a, mp4, mov, webm",
    modelLabel: "Model",
    modelHint: "Recommended: Turbo Q5 for best performance",
    engineLabel: "Engine",
    engineAuto: "Auto",
    engineMetal: "Metal",
    engineCpu: "CPU",
    engineHint: "Auto tries Metal first and falls back to CPU if needed",
    settingsSummary: "Settings",
    manageModels: "Manage",
    speakersLabel: "Identify speakers",
    speakerModelLabel: "Speaker model",
    speakersHint: "Automatically detect different speakers (max 2)",
    speakersOn: "On",
    speakersOff: "Off",
    experimentalWarning: "⚠️ Experimental feature",
    speakerSettings: "Settings",
    languageLabel: "Language",
    languageHint: "Use Multilingual Quality for mixed RU/EN content; Fast is quicker but less stable",
    decodingLabel: "Decoding",
    decodingHint: "Accurate uses beam search; slower but can reduce word distortions on noisy speech",
    decodingBalanced: "Balanced",
    decodingAccurate: "Accurate",
    autoDetected: "Auto-detected",
    vocabularyLabel: "Vocabulary / initial prompt",
    vocabularyHint: "Improves recognition of specific terms and names",
    vocabularyPlaceholder: "Names, terms, product titles",
    timestamps: "Timestamps",
    threadsLabel: "Threads",
    threadsAuto: "Auto",
    threadsManual: "Manual",
    threadsCount: "Count",
    transcribe: "Transcribe",
    cancel: "Cancel",
    transcribingHint: "Press Escape to cancel",
    transcript: "Transcript",
    segments: "segments",
    editableResult: "Editable result",
    copyAll: "Copy All",
    saveTxt: "Save TXT",
    saveMd: "Save MD",
    noModelError: "No local model is installed. Open Models and download Turbo Q5 or choose a custom .bin file.",
    speakerInfo: "Speaker labels are assigned after transcription completes.",
    copiedToClipboard: "Copied to clipboard!",
    noTranscriptToCopy: "There is no transcript to copy.",
    noTranscriptToExport: "There is no transcript to export.",
    benchmarkTitle: "Performance",
    benchmarkTotal: "Total",
    benchmarkAudio: "Audio",
    benchmarkRtf: "RTF",
    selectFileDifferent: "Click to choose a different file",
    selectFileOrDrag: "Click or drag file here",
    keyboardHint: "⌘ + O to open file",
    selectModelFirst: "Select a model",
    currentModel: "Current",
    openFile: "Open File",
    manageModelsTitle: "Model settings",
    settingsButton: "Settings",
    manageSpeakerModelsTitle: "Manage Speaker Models",
    modelName: "Model name",
    size: "Size",
    status: "Status",
    action: "Action",
    selected: "✓ Selected",
    installed: "✓ Installed",
    notInstalled: "Not installed",
    downloadingProgress: "Downloading",
    use: "Use",
    delete: "Delete",
    clear: "Clear",
    download: "Download",
    chooseFile: "Choose file",
    modelsDescription: "Stored in macOS application support. Custom files are linked, not copied.",
    close: "×"
  },
  ru: {
    appName: "LocalScribe",
    appDescription: "Локальная транскрибация аудио и видео. Без загрузки в облако.",
    stateLoading: "Загрузка",
    stateNoModel: "Нет модели",
    stateReady: "Готово",
    stateDownloading: "Загрузка",
    stateTranscribing: "Транскрибация",
    stateCompleted: "Завершено",
    stateError: "Ошибка",
    dropZoneDefault: "Перетащите аудио/видео сюда",
    dropZoneDragging: "Отпустите файл",
    dropZoneFormats: "aac, mp3, wav, m4a, mp4, mov, webm",
    modelLabel: "Модель",
    modelHint: "Рекомендуется: Turbo Q5 для лучшей производительности",
    engineLabel: "Движок",
    engineAuto: "Авто",
    engineMetal: "Metal",
    engineCpu: "CPU",
    engineHint: "Авто сначала пробует Metal и при необходимости откатывается на CPU",
    settingsSummary: "Настройки",
    manageModels: "Управление",
    settingsButton: "Настройки",
    speakersLabel: "Определять спикеров",
    speakerModelLabel: "Модель спикеров",
    speakersHint: "Автоматически определять разных говорящих (макс 2)",
    speakersOn: "Вкл",
    speakersOff: "Выкл",
    experimentalWarning: "⚠️ Экспериментальная функция",
    speakerSettings: "Настройки",
    languageLabel: "Язык",
    languageHint: "Для смешанного RU/EN используйте Multilingual Quality; Fast быстрее, но менее стабилен",
    decodingLabel: "Декодирование",
    decodingHint: "Точнее использует beam search; медленнее, но снижает искажение слов на шумной речи",
    decodingBalanced: "Сбалансировано",
    decodingAccurate: "Точнее",
    autoDetected: "Авто-определение",
    vocabularyLabel: "Словарь / начальный промпт",
    vocabularyHint: "Улучшает распознавание специфических терминов и имен",
    vocabularyPlaceholder: "Имена, термины, названия продуктов",
    timestamps: "Таймкоды",
    threadsLabel: "Потоки",
    threadsAuto: "Авто",
    threadsManual: "Вручную",
    threadsCount: "Количество",
    transcribe: "Транскрибировать",
    cancel: "Отмена",
    transcribingHint: "Нажмите Escape для отмены",
    transcript: "Транскрипт",
    segments: "сегментов",
    editableResult: "Редактируемый результат",
    copyAll: "Копировать всё",
    saveTxt: "Сохранить TXT",
    saveMd: "Сохранить MD",
    noModelError: "Нет локальной модели. Откройте Модели и скачайте Turbo Q5 или выберите кастомный .bin файл.",
    speakerInfo: "Метки спикеров присваиваются после завершения транскрибации.",
    copiedToClipboard: "Скопировано в буфер!",
    noTranscriptToCopy: "Нет транскрипта для копирования.",
    noTranscriptToExport: "Нет транскрипта для экспорта.",
    benchmarkTitle: "Производительность",
    benchmarkTotal: "Всего",
    benchmarkAudio: "Аудио",
    benchmarkRtf: "RTF",
    selectFileDifferent: "Нажмите чтобы выбрать другой файл",
    selectFileOrDrag: "Нажмите или перетащите файл",
    keyboardHint: "⌘ + O чтобы открыть файл",
    selectModelFirst: "Выберите модель",
    currentModel: "Текущая",
    openFile: "Открыть файл",
    manageModelsTitle: "Настройки модели",
    manageSpeakerModelsTitle: "Настройки модели спикеров",
    modelName: "Название модели",
    size: "Размер",
    status: "Статус",
    action: "Действие",
    selected: "✓ Выбрана",
    installed: "✓ Установлена",
    notInstalled: "Не установлена",
    downloadingProgress: "Загрузка",
    use: "Использовать",
    delete: "Удалить",
    clear: "Очистить",
    download: "Скачать",
    chooseFile: "Выбрать файл",
    modelsDescription: "Хранятся в служебной папке macOS. Кастомные файлы связаны, а не копируются.",
    close: "×"
  }
};

function detectSystemLanguage(): InterfaceLanguage {
  const systemLang = navigator.language || navigator.languages?.[0] || 'en';

  // Check if system language is Russian or includes Russian region
  if (systemLang.startsWith('ru') || systemLang.includes('RU')) {
    return 'ru';
  }

  // Default to English for all other languages
  return 'en';
}

function detectTranscriptionLanguage(): string {
  const systemLang = navigator.language || navigator.languages?.[0] || 'en';

  // Check if system language is Russian or includes Russian region
  if (systemLang.startsWith('ru') || systemLang.includes('RU')) {
    return 'ru';
  }

  // Default to English for all other languages
  return 'en';
}

function t(lang: InterfaceLanguage, key: string): string {
  return translations[lang]?.[key] || translations.en[key] || key;
}

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

function App() {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [speakerModels, setSpeakerModels] = useState<SpeakerModelInfo[]>([]);
  const [selectedModelId, setSelectedModelId] = useState("large-v3-turbo-q5_0");
  const [selectedSpeakerModelId, setSelectedSpeakerModelId] = useState("speaker-lite");
  const [speakerMode, setSpeakerMode] = useState<"off" | "auto">("off");
  const [engine, setEngine] = useState<"auto" | "metal" | "cpu">("auto");
  const [filePath, setFilePath] = useState("");
  const [language, setLanguage] = useState(() => detectTranscriptionLanguage());
  const [decodingPreset, setDecodingPreset] = useState<"balanced" | "accurate">("balanced");
  const [initialPrompt, setInitialPrompt] = useState("");
  const [timestamps, setTimestamps] = useState(true);
  const [threadsMode, setThreadsMode] = useState<"auto" | "manual">("auto");
  const [threads, setThreads] = useState(4);
  const [transcript, setTranscript] = useState("");
  const [segments, setSegments] = useState<TranscriptSegment[]>([]);
  const liveSegmentsRef = useRef<TranscriptSegment[]>([]);
  const [benchmark, setBenchmark] = useState<TranscriptionBenchmark | null>(null);
  const [progress, setProgress] = useState(0);
  const [phase, setPhase] = useState("");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [currentJobId, setCurrentJobId] = useState<string | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<Record<string, DownloadProgress>>({});
  const [speakerDownloadProgress, setSpeakerDownloadProgress] = useState<Record<string, DownloadProgress>>({});
  const [busyModelId, setBusyModelId] = useState<string | null>(null);
  const [busySpeakerModelId, setBusySpeakerModelId] = useState<string | null>(null);
  const [modelsOpen, setModelsOpen] = useState(false);
  const [speakerModelsOpen, setSpeakerModelsOpen] = useState(false);
  const [initializing, setInitializing] = useState(true);
  const [dragState, setDragState] = useState<DragState>({ isDragging: false, dragCounter: 0 });
  const [settingsOpen, setSettingsOpen] = useState(() => window.matchMedia("(min-width: 760px)").matches);
  const [interfaceLanguage, setInterfaceLanguage] = useState<InterfaceLanguage>(() => {
    // Try to load from localStorage first, otherwise detect from system
    const saved = localStorage.getItem('interfaceLanguage') as InterfaceLanguage | null;
    return saved || detectSystemLanguage();
  });

  const selectedModel = useMemo(
    () => models.find((model) => model.id === selectedModelId) ?? null,
    [models, selectedModelId]
  );
  const installedModels = models.filter((model) => model.installed);
  const installedSpeakerModels = speakerModels.filter((model) => model.installed);
  const hasUsableModel = Boolean(selectedModel?.installed && selectedModel.path);
  const transcribing = Boolean(currentJobId);
  const downloading = Boolean(busyModelId);
  const downloadingSpeakerModel = Boolean(busySpeakerModelId);

  // Helper function for translations
  const tx = (key: string) => t(interfaceLanguage, key);
  const uiState: UiState = initializing
    ? "loading"
    : transcribing
      ? "transcribing"
      : downloading
        ? "downloading"
        : error
          ? "error"
          : !installedModels.length
            ? "no-model"
            : transcript
              ? "completed"
              : "ready";

  useEffect(() => {
    // Save interface language to localStorage
    localStorage.setItem('interfaceLanguage', interfaceLanguage);
  }, [interfaceLanguage]);

  useEffect(() => {
    const query = window.matchMedia("(min-width: 760px)");
    const syncSettingsPanel = () => {
      if (query.matches) {
        setSettingsOpen(true);
      }
    };

    syncSettingsPanel();
    query.addEventListener("change", syncSettingsPanel);
    return () => query.removeEventListener("change", syncSettingsPanel);
  }, []);

  useEffect(() => {
    refreshModels();

    // Keyboard shortcuts
    const handleKeyPress = (e: KeyboardEvent) => {
      // Cmd/Ctrl + O: Open file
      if ((e.metaKey || e.ctrlKey) && e.key === 'o') {
        e.preventDefault();
        if (!transcribing) {
          chooseInputFile();
        }
      }

      // Cmd/Ctrl + Enter: Start transcription
      if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
        e.preventDefault();
        if (!transcribing && filePath && hasUsableModel) {
          startTranscription();
        }
      }

      // Escape: Cancel transcription or close modals
      if (e.key === 'Escape') {
        if (currentJobId) {
          cancelCurrentTranscription();
        }
        if (modelsOpen) setModelsOpen(false);
        if (speakerModelsOpen) setSpeakerModelsOpen(false);
      }

      // Cmd/Ctrl + M: Manage models
      if ((e.metaKey || e.ctrlKey) && e.key === 'm') {
        e.preventDefault();
        if (!transcribing) {
          setModelsOpen(true);
        }
      }
    };

    window.addEventListener('keydown', handleKeyPress);
    return () => window.removeEventListener('keydown', handleKeyPress);
  }, [transcribing, filePath, hasUsableModel, currentJobId, modelsOpen, speakerModelsOpen]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    const unlisteners: Array<() => void> = [];

    listen<DownloadProgress>("model-download-progress", (event) => {
      setDownloadProgress((previous) => ({
        ...previous,
        [event.payload.modelId]: event.payload
      }));
    }).then((unlisten) => unlisteners.push(unlisten));

    listen<DownloadProgress>("speaker-model-download-progress", (event) => {
      setSpeakerDownloadProgress((previous) => ({
        ...previous,
        [event.payload.modelId]: event.payload
      }));
    }).then((unlisten) => unlisteners.push(unlisten));

    listen<TranscriptionProgress>("transcription-progress", (event) => {
      if (currentJobId && event.payload.jobId !== currentJobId) {
        return;
      }
      setProgress(event.payload.progress);
      setPhase(event.payload.phase);
    }).then((unlisten) => unlisteners.push(unlisten));

    listen<TranscriptionSegmentEvent>("transcription-segment", (event) => {
      if (currentJobId && event.payload.jobId !== currentJobId) {
        return;
      }
      const segment = event.payload.segment;
      const filteredSegments = filterLiveTranscriptSegments(
        [...liveSegmentsRef.current.filter((item) => item.index !== segment.index), segment]
      );
      liveSegmentsRef.current = filteredSegments;
      setSegments(filteredSegments);
      setTranscript(formatTranscriptSegments(filteredSegments));
    }).then((unlisten) => unlisteners.push(unlisten));

    listen<TranscriptionError>("transcription-error", (event) => {
      if (currentJobId && event.payload.jobId !== currentJobId) {
        return;
      }
      setError(event.payload.message);
      setCurrentJobId(null);
      setPhase("error");
    }).then((unlisten) => unlisteners.push(unlisten));

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [currentJobId]);

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let unlisten: (() => void) | undefined;

    getCurrentWindow()
      .onDragDropEvent((event) => {
        const payload = event.payload as { type: string; paths?: string[] };
        if (payload.type === "drop" && payload.paths?.[0]) {
          setFilePath(payload.paths[0]);
          setError("");
          setDragState({ isDragging: false, dragCounter: 0 });
        }
      })
      .then((unsubscribe) => {
        unlisten = unsubscribe;
      });

    return () => unlisten?.();
  }, []);

  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragState(prev => ({ isDragging: true, dragCounter: prev.dragCounter + 1 }));
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragState(prev => {
      const newCounter = prev.dragCounter - 1;
      return { isDragging: newCounter > 0, dragCounter: newCounter };
    });
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragState({ isDragging: false, dragCounter: 0 });
  };

  async function refreshModels() {
    try {
      const [nextModels, nextSpeakerModels] = await Promise.all([
        listModels(),
        listSpeakerModels()
      ]);
      setModels(nextModels);
      setSpeakerModels(nextSpeakerModels);
      setSelectedModelId((current) => chooseSelectedModel(nextModels, current));
      setSelectedSpeakerModelId((current) => chooseSelectedSpeakerModel(nextSpeakerModels, current));
      setError("");
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setInitializing(false);
    }
  }

  async function chooseInputFile() {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Audio and video",
          extensions: ["aac", "mp3", "wav", "m4a", "mp4", "mov", "webm"]
        }
      ]
    });
    if (typeof selected === "string") {
      setFilePath(selected);
      setError("");
    }
  }

  async function handleDownload(modelId: string) {
    setBusyModelId(modelId);
    setError("");
    try {
      const model = await downloadModel(modelId);
      setSelectedModelId(model.id);
      await refreshModels();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusyModelId(null);
      setDownloadProgress((previous) => {
        const next = { ...previous };
        delete next[modelId];
        return next;
      });
    }
  }

  async function handleDelete(modelId: string) {
    setBusyModelId(modelId);
    setError("");
    try {
      const nextModels = await deleteModel(modelId);
      setModels(nextModels);
      setSelectedModelId((current) => chooseSelectedModel(nextModels, current));
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusyModelId(null);
    }
  }

  async function handleCustomModel(path: string) {
    setBusyModelId("custom");
    setError("");
    try {
      const model = await selectCustomModel(path);
      setSelectedModelId(model.id);
      await refreshModels();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusyModelId(null);
    }
  }

  async function handleSpeakerDownload(modelId: string) {
    setBusySpeakerModelId(modelId);
    setError("");
    try {
      const model = await downloadSpeakerModel(modelId);
      setSelectedSpeakerModelId(model.id);
      await refreshModels();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusySpeakerModelId(null);
      setSpeakerDownloadProgress((previous) => {
        const next = { ...previous };
        delete next[modelId];
        return next;
      });
    }
  }

  async function handleSpeakerDelete(modelId: string) {
    setBusySpeakerModelId(modelId);
    setError("");
    try {
      const nextModels = await deleteSpeakerModel(modelId);
      setSpeakerModels(nextModels);
      setSelectedSpeakerModelId((current) => chooseSelectedSpeakerModel(nextModels, current));
      if (!nextModels.some((model) => model.installed)) {
        setSpeakerMode("off");
      }
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusySpeakerModelId(null);
    }
  }

  async function handleCustomSpeakerModel(path: string) {
    setBusySpeakerModelId("custom-speaker");
    setError("");
    try {
      const model = await selectCustomSpeakerModel(path);
      setSelectedSpeakerModelId(model.id);
      await refreshModels();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusySpeakerModelId(null);
    }
  }

  async function startTranscription() {
    if (!filePath || !selectedModel?.path) {
      setError("Choose an input file and an installed model.");
      return;
    }

    const jobId = crypto.randomUUID();
    setCurrentJobId(jobId);
    setProgress(0);
    setPhase("starting");
    setError("");
    setTranscript("");
    setSegments([]);
    liveSegmentsRef.current = [];
    setBenchmark(null);

    try {
      const result = await transcribeFile({
        jobId,
        filePath,
        modelId: selectedModel.id,
        modelPath: selectedModel.path,
        engine,
        speakerMode,
        speakerModelId: speakerMode === "auto" ? selectedSpeakerModelId : undefined,
        language,
        decodingPreset,
        initialPrompt: initialPrompt.trim() || undefined,
        timestamps,
        threads: threadsMode === "manual" ? threads : undefined
      });
      liveSegmentsRef.current = result.segments;
      setTranscript(result.text);
      setSegments(result.segments);
      setBenchmark(result.benchmark ?? null);
      setProgress(100);
      setPhase("completed");
    } catch (caught) {
      setError(errorMessage(caught));
      setPhase("error");
    } finally {
      setCurrentJobId(null);
    }
  }

  async function cancelCurrentTranscription() {
    if (!currentJobId) {
      return;
    }
    await cancelTranscription(currentJobId);
    setPhase("cancelling");
  }

  async function saveTranscript(format: "txt" | "md") {
    if (!transcript.trim() && !segments.length) {
      setError(tx('noTranscriptToExport'));
      return;
    }

    const extension = format;
    const outputPath = await save({
      defaultPath: `transcript.${extension}`,
      filters: [{ name: format.toUpperCase(), extensions: [extension] }]
    });

    if (!outputPath) {
      return;
    }

    try {
      await exportTranscript({
        format,
        outputPath,
        text: transcript,
        segments
      });
      setError("");
    } catch (caught) {
      setError(errorMessage(caught));
    }
  }

  async function copyAllTranscript() {
    if (!transcript.trim() && !segments.length) {
      setNotice("");
      setError(tx('noTranscriptToCopy'));
      return;
    }

    try {
      await navigator.clipboard.writeText(transcript);
      const feedback = copySuccessFeedback(tx('copiedToClipboard'));
      setError(feedback.error);
      setNotice(feedback.notice);
      setTimeout(() => {
        setNotice("");
      }, 2000);
    } catch (caught) {
      const feedback = copyFailureFeedback(errorMessage(caught));
      setError(feedback.error);
      setNotice(feedback.notice);
    }
  }

  return (
    <main className="appShell">
      <header className="topBar">
        <div>
          <h1>{tx('appName')}</h1>
          <p>{tx('appDescription')}</p>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
          <button
            type="button"
            onClick={() => setInterfaceLanguage(interfaceLanguage === 'en' ? 'ru' : 'en')}
            title={interfaceLanguage === 'en' ? 'Switch to Russian' : 'Переключить на английский'}
            style={{ padding: '6px 12px', fontSize: '13px', fontWeight: '600' }}
          >
            {interfaceLanguage === 'en' ? 'РУС' : 'ENG'}
          </button>
          <div className={`stateBadge state-${uiState}`}>{tx(`state${uiState.charAt(0).toUpperCase() + uiState.slice(1)}`)}</div>
        </div>
      </header>

      <section className="workspace">
        <aside className="controlPane" aria-label="Transcription controls">
          <div className="keyboard-hint">
            <kbd>⌘</kbd> + <kbd>O</kbd> {tx('openFile')}
          </div>

          <button
            className={`dropZone ${dragState.isDragging ? 'dragging' : ''}`}
            type="button"
            onClick={chooseInputFile}
            disabled={transcribing}
            onDragEnter={handleDragEnter}
            onDragOver={handleDragOver}
            onDragLeave={handleDragLeave}
            onDrop={handleDrop}
            title={filePath ? tx('selectFileDifferent') : tx('selectFileOrDrag')}
          >
            <span>{filePath ? fileName(filePath) : (dragState.isDragging ? tx('dropZoneDragging') : tx('dropZoneDefault'))}</span>
            <small>{filePath || tx('dropZoneFormats')}</small>
          </button>

          <details
            className="settingsDisclosure"
            open={settingsOpen}
            onToggle={(event) => setSettingsOpen(event.currentTarget.open)}
          >
            <summary className="settingsSummary">
              <span>{tx('settingsSummary')}</span>
              <strong>{selectedModel?.name ?? tx('selectModelFirst')}</strong>
            </summary>
            <div className="settingsBody">
              <label className="field">
                <span className="hint" data-hint={tx('modelHint')}>
                  {tx('modelLabel')}
                </span>
                <div className="inlineField">
                  <select
                    value={selectedModelId}
                    onChange={(event) => setSelectedModelId(event.target.value)}
                    disabled={transcribing}
                    title={selectedModel ? `${tx('currentModel')}: ${selectedModel.name}` : tx('selectModelFirst')}
                  >
                    {models.map((model) => (
                      <option value={model.id} key={model.id} disabled={!model.installed}>
                        {model.name} {model.installed ? "" : `(${tx('notInstalled')})`}
                      </option>
                    ))}
                  </select>
                  <button
                    type="button"
                    className="settingsButton"
                    onClick={() => setModelsOpen(true)}
                    disabled={transcribing}
                    title={tx('manageModelsTitle')}
                  >
                    {interfaceLanguage === 'en' ? 'Settings' : 'Настройки'}
                  </button>
                </div>
              </label>

              <label className="field">
                <span className="hint" data-hint={tx('engineHint')}>
                  {tx('engineLabel')}
                </span>
                <select
                  value={engine}
                  onChange={(event) => setEngine(event.target.value as "auto" | "metal" | "cpu")}
                  disabled={transcribing}
                >
                  <option value="auto">{tx('engineAuto')}</option>
                  <option value="metal">{tx('engineMetal')}</option>
                  <option value="cpu">{tx('engineCpu')}</option>
                </select>
              </label>

              <label className="field">
                <div className="speakerField">
                  <span className="hint" data-hint={tx('speakersHint')}>
                    {tx('speakersLabel')}
                  </span>
                  <button
                    type="button"
                    className="settingsButton"
                    onClick={() => setSpeakerModelsOpen(true)}
                    disabled={transcribing}
                    title={tx('manageSpeakerModelsTitle')}
                  >
                    {tx('settingsButton')}
                  </button>
                </div>
                <div className="speakerSwitch">
                  <button
                    type="button"
                    className={`switch ${speakerMode === "auto" ? "active" : ""}`}
                    onClick={() => {
                      if (speakerMode === "off" && installedSpeakerModels.length > 0) {
                        setSpeakerMode("auto");
                      } else if (speakerMode === "auto") {
                        setSpeakerMode("off");
                      }
                    }}
                    disabled={transcribing || (speakerMode === "off" && !installedSpeakerModels.length)}
                    title={speakerMode === "auto" ? "Click to disable" : "Click to enable"}
                  >
                    <span className="slider"></span>
                  </button>
                  <span className="switchLabel">
                    {speakerMode === "auto" ? tx('speakersOn') : tx('speakersOff')}
                  </span>
                </div>
                {speakerMode === "auto" && !transcribing && (
                  <small className="field-hint">
                    {tx('experimentalWarning')}
                  </small>
                )}
              </label>

              {speakerMode === "auto" ? (
                <label className="field">
                  <span>{tx('speakerModelLabel')}</span>
                  <select
                    value={selectedSpeakerModelId}
                    onChange={(event) => setSelectedSpeakerModelId(event.target.value)}
                    disabled={transcribing || downloadingSpeakerModel}
                  >
                    {speakerModels.map((model) => (
                      <option value={model.id} key={model.id} disabled={!model.installed}>
                        {model.name} {model.installed ? "" : `(${tx('notInstalled')})`}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}

              <label className="field">
                <span>{tx('languageLabel')}</span>
                <select
                  value={language}
                  onChange={(event) => setLanguage(event.target.value)}
                  disabled={transcribing}
                >
                  {languages.map((item) => (
                    <option value={item.value} key={item.value}>
                      {item.label}
                    </option>
                  ))}
                </select>
              </label>

              <label className="field">
                <span className="hint" data-hint={tx('decodingHint')}>
                  {tx('decodingLabel')}
                </span>
                <select
                  value={decodingPreset}
                  onChange={(event) => setDecodingPreset(event.target.value as "balanced" | "accurate")}
                  disabled={transcribing}
                >
                  <option value="balanced">{tx('decodingBalanced')}</option>
                  <option value="accurate">{tx('decodingAccurate')}</option>
                </select>
              </label>

              <label className="field wideField">
                <span className="hint" data-hint={tx('vocabularyHint')}>
                  {tx('vocabularyLabel')}
                </span>
                <textarea
                  className="promptField"
                  value={initialPrompt}
                  onChange={(event) => setInitialPrompt(event.target.value)}
                  placeholder={tx('vocabularyPlaceholder')}
                  disabled={transcribing}
                />
              </label>

              <div className="settingsGrid wideField">
                <label className="checkField">
                  <input
                    type="checkbox"
                    checked={timestamps}
                    onChange={(event) => setTimestamps(event.target.checked)}
                    disabled={transcribing}
                  />
                  <span>{tx('timestamps')}</span>
                </label>

                <label className="field compact">
                  <span>{tx('threadsLabel')}</span>
                  <select
                    value={threadsMode}
                    onChange={(event) => setThreadsMode(event.target.value as "auto" | "manual")}
                    disabled={transcribing}
                  >
                    <option value="auto">{tx('threadsAuto')}</option>
                    <option value="manual">{tx('threadsManual')}</option>
                  </select>
                </label>

                {threadsMode === "manual" ? (
                  <label className="field compact">
                    <span>{tx('threadsCount')}</span>
                    <input
                      type="number"
                      min={1}
                      max={32}
                      value={threads}
                      onChange={(event) => setThreads(Number(event.target.value))}
                      disabled={transcribing}
                    />
                  </label>
                ) : null}
              </div>
            </div>
          </details>

          {transcribing ? (
            <div className="transcribing-hint">
              {tx('transcribingHint')}
            </div>
          ) : (
            <button
              className="primary"
              type="button"
              onClick={startTranscription}
              disabled={!filePath || !hasUsableModel || initializing || downloading}
            >
              {tx('transcribe')}
            </button>
          )}

          <div className="progressBlock" aria-live="polite">
            <div className="progressMeta">
              <span className={`phaseLabel ${uiState}`}>
                {phase || tx(`state${uiState.charAt(0).toUpperCase() + uiState.slice(1)}`)}
              </span>
              <span>{Math.round(progress)}%</span>
            </div>
            <progress
              max={100}
              value={progress}
            />
            {transcribing && (
              <div className="progressActions">
                <button
                  type="button"
                  onClick={cancelCurrentTranscription}
                  className="danger"
                  title={tx('transcribingHint')}
                >
                  {tx('cancel')}
                </button>
              </div>
            )}
          </div>

          {benchmark ? (
            <details className="benchmarkBox" aria-label={tx('benchmarkTitle')}>
              <summary className="benchmarkHeader">
                <div className="benchmarkTitleLine">
                  <span>{tx('benchmarkTitle')}</span>
                  <strong>{tx('benchmarkTotal')}: {formatDuration(benchmark.totalMs)}</strong>
                </div>
                <div className="benchmarkSummary">
                  <span>{tx('benchmarkAudio')}: {formatDuration(benchmark.audioDurationMs)}</span>
                  <span>{tx('benchmarkRtf')}: {formatRtf(benchmark.totalMs, benchmark.audioDurationMs)}</span>
                </div>
              </summary>
              <div className="benchmarkRows">
                {benchmark.phases.map((item: BenchmarkPhase, index: number) => (
                  <div className="benchmarkRow" key={`${item.phase}-${index}`}>
                    <span>{benchmarkPhaseLabel(item.phase, interfaceLanguage)}</span>
                    <strong>{formatDuration(item.durationMs)}</strong>
                  </div>
                ))}
              </div>
            </details>
          ) : null}

          {notice ? <div className="noticeBox">{notice}</div> : null}
          {error ? <div className="errorBox">{error}</div> : null}
          {!installedModels.length && !initializing ? (
            <div className="emptyBox">
              {tx('noModelError')}
            </div>
          ) : null}
          {speakerMode === "auto" ? (
            <div className="emptyBox">
              {tx('speakerInfo')}
            </div>
          ) : null}
        </aside>

        <section className="editorPane" aria-label="Transcript editor">
          <div className="editorHeader">
            <div>
              <h2>{tx('transcript')}</h2>
              <p>{segments.length ? `${segments.length} ${tx('segments')}` : tx('editableResult')}</p>
            </div>
            <div className="exportActions">
              <button
                type="button"
                onClick={copyAllTranscript}
                disabled={!transcript}
                title="Copy all text to clipboard"
              >
                {tx('copyAll')}
              </button>
              <button type="button" onClick={() => saveTranscript("txt")} disabled={!transcript}>
                {tx('saveTxt')}
              </button>
              <button type="button" onClick={() => saveTranscript("md")} disabled={!transcript}>
                {tx('saveMd')}
              </button>
            </div>
          </div>

          <textarea
            className="transcriptEditor"
            value={transcript}
            onChange={(event) => setTranscript(event.target.value)}
            placeholder="Transcript will appear here."
          />
        </section>
      </section>

      {modelsOpen ? (
        <ModelsModal
          models={models}
          selectedModelId={selectedModelId}
          downloadProgress={downloadProgress}
          busyModelId={busyModelId}
          onClose={() => setModelsOpen(false)}
          onUse={(model) => {
            setSelectedModelId(model.id);
            setModelsOpen(false);
          }}
          onDownload={handleDownload}
          onDelete={handleDelete}
          onChooseCustom={handleCustomModel}
          interfaceLanguage={interfaceLanguage}
        />
      ) : null}

      {speakerModelsOpen ? (
        <SpeakerModelsModal
          models={speakerModels}
          selectedModelId={selectedSpeakerModelId}
          downloadProgress={speakerDownloadProgress}
          busyModelId={busySpeakerModelId}
          onClose={() => setSpeakerModelsOpen(false)}
          onUse={(model) => {
            setSelectedSpeakerModelId(model.id);
            setSpeakerModelsOpen(false);
            setSpeakerMode("auto");
          }}
          onDownload={handleSpeakerDownload}
          onDelete={handleSpeakerDelete}
          onChooseCustom={handleCustomSpeakerModel}
        />
      ) : null}
    </main>
  );
}

function chooseSelectedModel(models: ModelInfo[], current: string): string {
  if (models.some((model) => model.id === current && model.installed)) {
    return current;
  }
  return (
    models.find((model) => model.installed && model.recommended)?.id ??
    models.find((model) => model.installed)?.id ??
    current
  );
}

function chooseSelectedSpeakerModel(models: SpeakerModelInfo[], current: string): string {
  if (models.some((model) => model.id === current && model.installed)) {
    return current;
  }
  return (
    models.find((model) => model.installed && model.recommended)?.id ??
    models.find((model) => model.installed)?.id ??
    current
  );
}

function fileName(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

function formatDuration(ms: number): string {
  if (ms < 1_000) {
    return `${Math.round(ms)} ms`;
  }

  const seconds = ms / 1_000;
  if (seconds < 60) {
    return `${seconds.toFixed(seconds < 10 ? 1 : 0)} s`;
  }

  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.round(seconds % 60);
  return `${minutes}m ${remainingSeconds.toString().padStart(2, "0")}s`;
}

function formatRtf(totalMs: number, audioDurationMs: number): string {
  if (!audioDurationMs) {
    return "-";
  }
  return `${(totalMs / audioDurationMs).toFixed(2)}x`;
}

function benchmarkPhaseLabel(phase: string, language: InterfaceLanguage): string {
  const labels: Record<InterfaceLanguage, Record<string, string>> = {
    en: {
      "audio-preparation": "Audio preparation",
      "model-load-metal": "Load model (Metal)",
      "model-load-cpu": "Load model (CPU)",
      "whisper-metal": "Whisper (Metal)",
      "whisper-cpu": "Whisper (CPU)",
      "whisper-chunked-metal": "Whisper chunked (Metal)",
      "whisper-chunked-cpu": "Whisper chunked (CPU)",
      "speaker-assignment": "Speaker labels"
    },
    ru: {
      "audio-preparation": "Подготовка аудио",
      "model-load-metal": "Загрузка модели (Metal)",
      "model-load-cpu": "Загрузка модели (CPU)",
      "whisper-metal": "Whisper (Metal)",
      "whisper-cpu": "Whisper (CPU)",
      "whisper-chunked-metal": "Whisper чанками (Metal)",
      "whisper-chunked-cpu": "Whisper чанками (CPU)",
      "speaker-assignment": "Метки спикеров"
    }
  };

  return labels[language][phase] ?? phase;
}

function stateLabel(state: UiState): string {
  switch (state) {
    case "loading":
      return "Loading";
    case "no-model":
      return "No model";
    case "ready":
      return "Ready";
    case "downloading":
      return "Downloading";
    case "transcribing":
      return "Transcribing";
    case "completed":
      return "Completed";
    case "error":
      return "Error";
  }
}

export default App;
