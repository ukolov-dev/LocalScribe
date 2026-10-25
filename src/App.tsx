import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open, save } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useRef, useState } from "react";
import { ModelsModal } from "./components/ModelsModal";
import { SpeakerModelsModal } from "./components/SpeakerModelsModal";
import {
  cancelTranscription,
  claimIntegrationRequest,
  completeIntegrationRequest,
  deleteModel,
  deleteSpeakerModel,
  downloadModel,
  downloadSpeakerModel,
  errorMessage,
  exportTranscript,
  failIntegrationRequest,
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
  IntegrationRequest,
  ModelInfo,
  SpeakerModelInfo,
  TranscriptionBenchmark,
  TranscriptSegment,
  TranscriptionError,
  TranscriptionProgress,
  TranscriptionResult,
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
    autoStartHint: "Transcription starts automatically",
    alwaysSpeakers: "Speakers are always identified",
    speakerLimit: "Neural diarization · automatic speaker count",
    unsupportedFile: "Choose an audio or video file: aac, mp3, wav, m4a, mp4, mov, webm.",
    oneFileOnly: "Drop one recording at a time.",
    retry: "Try again",
    transcriptPlaceholder: "Your transcript with speaker labels will appear here.",
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
    speakersHint: "Automatically detect different speakers",
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
    copyAll: "Copy",
    saveTxt: "Save TXT",
    saveMd: "Save MD",
    noModelError: "No local model is installed. Open Models and download Turbo Q5 or choose a custom .bin file.",
    speakerInfo: "Speaker labels are assigned after transcription completes.",
    copiedToClipboard: "Copied to clipboard!",
    noTranscriptToCopy: "There is no transcript to copy.",
    noTranscriptToExport: "There is no transcript to export.",
    expressImportComplete: "eXpress meeting transcribed",
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
    autoStartHint: "Расшифровка начнётся автоматически",
    alwaysSpeakers: "Спикеры определяются всегда",
    speakerLimit: "Нейросеть · число голосов автоматически",
    unsupportedFile: "Выберите аудио или видео: aac, mp3, wav, m4a, mp4, mov, webm.",
    oneFileOnly: "Перетаскивайте по одной записи.",
    retry: "Повторить",
    transcriptPlaceholder: "Здесь появится текст со спикерами.",
    appName: "LocalScribe",
    appDescription: "Локальная транскрибация аудио и видео. Без загрузки в облако.",
    stateLoading: "Загрузка",
    stateNoModel: "Нет модели",
    stateReady: "Готово",
    stateDownloading: "Загрузка",
    stateTranscribing: "Транскрибация",
    stateCompleted: "Завершено",
    stateError: "Ошибка",
    dropZoneDefault: "Перетащите запись",
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
    speakersHint: "Автоматически определять разных говорящих",
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
    copyAll: "Копировать",
    saveTxt: "Сохранить TXT",
    saveMd: "Сохранить MD",
    noModelError: "Нет локальной модели. Откройте Модели и скачайте Turbo Q5 или выберите кастомный .bin файл.",
    speakerInfo: "Метки спикеров присваиваются после завершения транскрибации.",
    copiedToClipboard: "Скопировано в буфер!",
    noTranscriptToCopy: "Нет транскрипта для копирования.",
    noTranscriptToExport: "Нет транскрипта для экспорта.",
    expressImportComplete: "Встреча eXpress расшифрована",
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
  const [engine, setEngine] = useState<"auto" | "metal" | "cpu">("auto");
  const [filePath, setFilePath] = useState("");
  const [pendingFilePath, setPendingFilePath] = useState<string | null>(null);
  const [language, setLanguage] = useState(() => detectTranscriptionLanguage());
  const [decodingPreset, setDecodingPreset] = useState<"balanced" | "accurate">("balanced");
  const [initialPrompt, setInitialPrompt] = useState("");
  const [timestamps, setTimestamps] = useState(true);
  const [threadsMode, setThreadsMode] = useState<"auto" | "manual">("auto");
  const [threads, setThreads] = useState(4);
  const [transcript, setTranscript] = useState("");
  const [segments, setSegments] = useState<TranscriptSegment[]>([]);
  const liveSegmentsRef = useRef<TranscriptSegment[]>([]);
  const integrationBusyRef = useRef(false);
  const [checkingIntegration, setCheckingIntegration] = useState(false);
  const transcriptionBusyRef = useRef(false);
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
  const [settingsOpen, setSettingsOpen] = useState(false);
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
  const hasUsableModel = Boolean(selectedModel?.installed && selectedModel.path);
  const transcribing = Boolean(currentJobId);
  const downloading = Boolean(busyModelId);

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
    void refreshModels();
  }, []);

  useEffect(() => {
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

    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const registerUnlisten = (unlisten: () => void) => {
      if (disposed) unlisten();
      else unlisteners.push(unlisten);
    };
    const onListenError = (caught: unknown) => {
      if (!disposed) setError(errorMessage(caught));
    };

    listen<DownloadProgress>("model-download-progress", (event) => {
      if (disposed) return;
      setDownloadProgress((previous) => ({
        ...previous,
        [event.payload.modelId]: event.payload
      }));
    }).then(registerUnlisten).catch(onListenError);

    listen<DownloadProgress>("speaker-model-download-progress", (event) => {
      if (disposed) return;
      setSpeakerDownloadProgress((previous) => ({
        ...previous,
        [event.payload.modelId]: event.payload
      }));
    }).then(registerUnlisten).catch(onListenError);

    listen<TranscriptionProgress>("transcription-progress", (event) => {
      if (disposed || event.payload.jobId !== currentJobId) {
        return;
      }
      setProgress(event.payload.progress);
      setPhase(event.payload.phase);
    }).then(registerUnlisten).catch(onListenError);

    listen<TranscriptionSegmentEvent>("transcription-segment", (event) => {
      if (disposed || event.payload.jobId !== currentJobId) {
        return;
      }
      const segment = event.payload.segment;
      liveSegmentsRef.current = [
        ...liveSegmentsRef.current.filter((item) => item.index !== segment.index), segment
      ];
      const filteredSegments = filterLiveTranscriptSegments(liveSegmentsRef.current);
      setSegments(filteredSegments);
      setTranscript(formatTranscriptSegments(filteredSegments));
    }).then(registerUnlisten).catch(onListenError);

    listen<TranscriptionError>("transcription-error", (event) => {
      if (disposed || event.payload.jobId !== currentJobId) {
        return;
      }
      setError(event.payload.message);
      setCurrentJobId(null);
      setPhase("error");
    }).then(registerUnlisten).catch(onListenError);

    return () => {
      disposed = true;
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [currentJobId]);

  const acceptInputRef = useRef(acceptInputFile);
  acceptInputRef.current = acceptInputFile;

  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }

    let disposed = false;
    let unlisten: (() => void) | undefined;

    getCurrentWindow()
      .onDragDropEvent((event) => {
        if (disposed) return;
        const payload = event.payload as { type: string; paths?: string[] };
        setDragState({ isDragging: payload.type === "enter" || payload.type === "over", dragCounter: 0 });
        if (payload.type === "drop" && payload.paths?.[0]) {
          if (payload.paths.length !== 1) {
            acceptInputRef.current('');
            return;
          }
          acceptInputRef.current(payload.paths[0]);
        }
      })
      .then((unsubscribe) => {
        if (disposed) unsubscribe();
        else unlisten = unsubscribe;
      }).catch((caught) => {
        if (!disposed) setError(errorMessage(caught));
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!pendingFilePath || initializing || downloading || checkingIntegration || transcribing || !hasUsableModel || transcriptionBusyRef.current) return;
    setPendingFilePath(null);
    void startTranscription(pendingFilePath);
  }, [pendingFilePath, initializing, downloading, checkingIntegration, transcribing, hasUsableModel, selectedModel?.path]);

  useEffect(() => {
    if (!isTauriRuntime() || initializing || transcribing || pendingFilePath || !hasUsableModel) {
      return;
    }

    let disposed = false;
    const poll = async () => {
      if (disposed || integrationBusyRef.current || transcriptionBusyRef.current) {
        return;
      }

      integrationBusyRef.current = true;
      setCheckingIntegration(true);
      try {
        const request = await claimIntegrationRequest();
        if (request && !disposed) {
          await startIntegrationTranscription(request);
        }
      } catch (caught) {
        if (!disposed) {
          setError(errorMessage(caught));
        }
      } finally {
        integrationBusyRef.current = false;
        setCheckingIntegration(false);
      }
    };

    void poll();
    const timer = window.setInterval(() => void poll(), 2000);
    return () => {
      disposed = true;
      window.clearInterval(timer);
    };
  }, [
    initializing,
    pendingFilePath,
    transcribing,
    hasUsableModel,
    selectedModel?.id,
    selectedModel?.path,
    engine,
    language,
    decodingPreset,
    initialPrompt,
    timestamps,
    threadsMode,
    threads,
    interfaceLanguage
  ]);

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
      setError("");
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setInitializing(false);
    }
  }

  function acceptInputFile(path: string) {
    if (transcriptionBusyRef.current) return;
    if (!path) {
      setError(tx('oneFileOnly'));
      return;
    }
    if (!/\.(aac|mp3|wav|m4a|mp4|mov|webm)$/i.test(path)) {
      setError(tx('unsupportedFile'));
      return;
    }
    setFilePath(path);
    setPendingFilePath(path);
    setError(hasUsableModel ? "" : tx('noModelError'));
    setNotice("");
    if (!hasUsableModel && !initializing) setModelsOpen(true);
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
      acceptInputFile(selected);
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
      await downloadSpeakerModel(modelId);
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
      await selectCustomSpeakerModel(path);
      await refreshModels();
    } catch (caught) {
      setError(errorMessage(caught));
    } finally {
      setBusySpeakerModelId(null);
    }
  }

  async function startTranscription(inputPath = filePath) {
    if (!inputPath || !selectedModel?.path) {
      setError("Choose an input file and an installed model.");
      return;
    }
    if (transcriptionBusyRef.current) {
      return;
    }
    transcriptionBusyRef.current = true;

    const jobId = crypto.randomUUID();
    setCurrentJobId(jobId);
    setProgress(0);
    setPhase("starting");
    setError("");
    setNotice("");
    setTranscript("");
    setSegments([]);
    liveSegmentsRef.current = [];
    setBenchmark(null);

    try {
      const result = await transcribeFile({
        jobId,
        filePath: inputPath,
        modelId: selectedModel.id,
        modelPath: selectedModel.path,
        engine,
        speakerMode: "auto",
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
      transcriptionBusyRef.current = false;
      setCurrentJobId(null);
    }
  }

  async function startIntegrationTranscription(request: IntegrationRequest) {
    if (!selectedModel?.path || transcriptionBusyRef.current) {
      return;
    }
    transcriptionBusyRef.current = true;

    const jobId = request.id;
    setFilePath(request.mediaPath);
    setCurrentJobId(jobId);
    setProgress(0);
    setPhase("starting");
    setError("");
    setNotice("");
    setTranscript("");
    setSegments([]);
    liveSegmentsRef.current = [];
    setBenchmark(null);

    try {
      const result: TranscriptionResult = await transcribeFile({
        jobId,
        filePath: request.mediaPath,
        modelId: selectedModel.id,
        modelPath: selectedModel.path,
        engine,
        speakerMode: "auto",
        language: request.options?.language ?? language,
        decodingPreset: request.options?.decodingPreset ?? decodingPreset,
        initialPrompt: initialPrompt.trim() || undefined,
        timestamps: request.options?.timestamps ?? timestamps,
        threads: threadsMode === "manual" ? threads : undefined
      });
      const completion = await completeIntegrationRequest(jobId, result);
      liveSegmentsRef.current = result.segments;
      setTranscript(result.text);
      setSegments(result.segments);
      setBenchmark(result.benchmark ?? null);
      setProgress(100);
      setPhase("completed");
      setNotice(`${tx('expressImportComplete')}: ${completion.transcriptPath}`);
    } catch (caught) {
      const message = errorMessage(caught);
      setError(message);
      setPhase("error");
      try {
        await failIntegrationRequest(jobId, message);
      } catch (failError) {
        setError(`${message} ${errorMessage(failError)}`);
      }
    } finally {
      transcriptionBusyRef.current = false;
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
            <svg className="uploadIcon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
              <path d="M12 16V3m-5 5 5-5 5 5M4 15v5a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-5" />
            </svg>
            <span>{dragState.isDragging ? tx('dropZoneDragging') : filePath ? fileName(filePath) : tx('dropZoneDefault')}</span>
            <small>{transcribing ? tx('transcribingHint') : tx('autoStartHint')}</small>
            <small>{tx('dropZoneFormats')}</small>
          </button>

          <div className="speakerStatus">
            <span>{tx('alwaysSpeakers')}</span>
            <small>{tx('speakerLimit')}</small>
          </div>

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
              <button type="button" className="settingsButton wideField" disabled={transcribing} onClick={() => setSpeakerModelsOpen(true)}>
                {tx('manageSpeakerModelsTitle')}
              </button>
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

          {filePath && !transcribing && !pendingFilePath && phase === "error" ? (
            <button
              className="primary"
              type="button"
              onClick={() => void startTranscription()}
              disabled={!filePath || !hasUsableModel || initializing || downloading}
            >
              {tx('retry')}
            </button>
          ) : null}

          {transcribing ? <div className="progressBlock" aria-live="polite">
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
          </div> : null}

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
                TXT
              </button>
              <button type="button" onClick={() => saveTranscript("md")} disabled={!transcript}>
                MD
              </button>
            </div>
          </div>

          <textarea
            className="transcriptEditor"
            value={transcript}
            readOnly={transcribing}
            onChange={(event) => {
              setTranscript(event.target.value);
              setSegments([]);
            }}
            aria-label={tx('transcript')}
            placeholder={tx('transcriptPlaceholder')}
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
          downloadProgress={speakerDownloadProgress}
          busyModelId={busySpeakerModelId}
          onClose={() => setSpeakerModelsOpen(false)}
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

export default App;
