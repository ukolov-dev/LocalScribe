import { open } from "@tauri-apps/plugin-dialog";
import type { DownloadProgress, ModelInfo } from "../types/localscribe";

type ModelsModalProps = {
  models: ModelInfo[];
  selectedModelId: string;
  downloadProgress: Record<string, DownloadProgress>;
  busyModelId: string | null;
  onClose: () => void;
  onUse: (model: ModelInfo) => void;
  onDownload: (modelId: string) => void;
  onDelete: (modelId: string) => void;
  onChooseCustom: (path: string) => void;
  interfaceLanguage: "en" | "ru";
};

const translations: Record<"en" | "ru", Record<string, string>> = {
  en: {
    title: "Models",
    description: "Choose a speed/quality profile. Built-in downloads are pinned and SHA-256 verified; larger models need more memory.",
    modelName: "Model name",
    size: "Size",
    status: "Status",
    action: "Action",
    selected: "✓ Selected",
    installed: "✓ Installed",
    notInstalled: "Not installed",
    downloading: "Downloading",
    chooseFile: "Choose file",
    use: "Use",
    delete: "Delete",
    clear: "Clear",
    download: "Download",
    close: "×"
  },
  ru: {
    title: "Модели",
    description: "Выберите баланс скорости и качества. Встроенные загрузки закреплены и проверяются по SHA-256; крупным моделям нужно больше памяти.",
    modelName: "Название модели",
    size: "Размер",
    status: "Статус",
    action: "Действие",
    selected: "✓ Выбрана",
    installed: "✓ Установлена",
    notInstalled: "Не установлена",
    downloading: "Загрузка",
    chooseFile: "Выбрать файл",
    use: "Использовать",
    delete: "Удалить",
    clear: "Очистить",
    download: "Скачать",
    close: "×"
  }
};

function t(lang: "en" | "ru", key: string): string {
  return translations[lang]?.[key] || translations.en[key] || key;
}

export function ModelsModal({
  models,
  selectedModelId,
  downloadProgress,
  busyModelId,
  onClose,
  onUse,
  onDownload,
  onDelete,
  onChooseCustom,
  interfaceLanguage
}: ModelsModalProps) {
  const tx = (key: string) => t(interfaceLanguage, key);
  async function chooseCustomModel() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Whisper model", extensions: ["bin"] }]
    });

    if (typeof selected === "string") {
      onChooseCustom(selected);
    }
  }

  return (
    <div className="modalBackdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label={tx('title')}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="modalHeader">
          <div>
            <h2>{tx('title')}</h2>
            <p>{tx('description')}</p>
          </div>
          <button className="iconButton" type="button" onClick={onClose} title={tx('close')}>
            {tx('close')}
          </button>
        </header>

        <div className="modelTable" role="table" aria-label="Model manager">
          <div className="modelRow modelHead" role="row">
            <span>{tx('modelName')}</span>
            <span>{tx('size')}</span>
            <span>{tx('status')}</span>
            <span>{tx('action')}</span>
          </div>

          {models.map((model) => {
            const progress = downloadProgress[model.id];
            const busy = busyModelId === model.id;
            const status = progress
              ? `${tx('downloading')} ${Math.round(progress.progress)}%`
              : model.installed
                ? selectedModelId === model.id
                  ? tx('selected')
                  : tx('installed')
                : tx('notInstalled');

            const statusClass = progress
              ? "downloading"
              : model.installed
                ? "installed"
                : "not-installed";

            return (
              <div
                className={`modelRow ${statusClass}`}
                role="row"
                key={model.id}
                title={model.custom && model.path ? model.path : undefined}
              >
                <span>
                  <strong>{model.name}</strong>
                  <small>{model.qualityLabel}</small>
                </span>
                <span>{model.sizeLabel}</span>
                <span>
                  <span className={`status-badge ${statusClass}`}>
                    {status}
                  </span>
                  {progress ? (
                    <progress
                      className="inlineProgress"
                      max={100}
                      value={progress.progress}
                      title={`Downloaded: ${Math.round(progress.downloadedBytes / 1024 / 1024)} MB / ${Math.round((progress.totalBytes || progress.downloadedBytes) / 1024 / 1024)} MB`}
                    />
                  ) : null}
                </span>
                <span className="rowActions">
                  {model.custom ? (
                    <button type="button" onClick={chooseCustomModel} disabled={busy}>
                      {tx('chooseFile')}
                    </button>
                  ) : model.installed ? (
                    <>
                      <button type="button" onClick={() => onUse(model)}>
                        {tx('use')}
                      </button>
                      <button type="button" onClick={() => onDelete(model.id)} disabled={busy}>
                        {tx('delete')}
                      </button>
                    </>
                  ) : (
                    <button type="button" onClick={() => onDownload(model.id)} disabled={busy}>
                      {tx('download')}
                    </button>
                  )}

                  {model.custom && model.installed ? (
                    <>
                      <button type="button" onClick={() => onUse(model)}>
                        {tx('use')}
                      </button>
                      <button type="button" onClick={() => onDelete(model.id)} disabled={busy}>
                        {tx('clear')}
                      </button>
                    </>
                  ) : null}
                </span>
              </div>
            );
          })}
        </div>
      </section>
    </div>
  );
}
