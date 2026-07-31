import { open } from "@tauri-apps/plugin-dialog";
import type { DownloadProgress, SpeakerModelInfo } from "../types/localscribe";

type SpeakerModelsModalProps = {
  models: SpeakerModelInfo[];
  selectedModelId: string;
  downloadProgress: Record<string, DownloadProgress>;
  busyModelId: string | null;
  onClose: () => void;
  onUse: (model: SpeakerModelInfo) => void;
  onDownload: (modelId: string) => void;
  onDelete: (modelId: string) => void;
  onChooseCustom: (path: string) => void;
};

export function SpeakerModelsModal({
  models,
  selectedModelId,
  downloadProgress,
  busyModelId,
  onClose,
  onUse,
  onDownload,
  onDelete,
  onChooseCustom
}: SpeakerModelsModalProps) {
  async function chooseCustomModel() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Speaker ONNX model", extensions: ["onnx"] }]
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
        aria-label="Speaker models"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="modalHeader">
          <div>
            <h2>Speaker models</h2>
            <p>VAD and speaker embedding model packs stored locally.</p>
          </div>
          <button className="iconButton" type="button" onClick={onClose} title="Close">
            x
          </button>
        </header>

        <div className="modelTable" role="table" aria-label="Speaker model manager">
          <div className="modelRow modelHead" role="row">
            <span>Model name</span>
            <span>Size</span>
            <span>Status</span>
            <span>Action</span>
          </div>

          {models.map((model) => {
            const progress = downloadProgress[model.id];
            const busy = busyModelId === model.id;
            const status = progress
              ? `Downloading ${Math.round(progress.progress)}%`
              : model.installed
                ? selectedModelId === model.id
                  ? "Selected"
                  : "Installed"
                : "Not installed";

            return (
              <div className="modelRow" role="row" key={model.id}>
                <span>
                  <strong>{model.name}</strong>
                  <small>{model.qualityLabel}</small>
                  <small>{model.runtimeLabel}</small>
                  {model.files.length ? (
                    <small>{model.files.map((file) => file.fileName).join(", ")}</small>
                  ) : null}
                </span>
                <span>{model.sizeLabel}</span>
                <span>
                  {status}
                  {progress ? (
                    <progress className="inlineProgress" max={100} value={progress.progress} />
                  ) : null}
                </span>
                <span className="rowActions">
                  {model.custom ? (
                    <button type="button" onClick={chooseCustomModel} disabled={busy}>
                      Choose file
                    </button>
                  ) : model.installed ? (
                    <>
                      <button type="button" onClick={() => onUse(model)}>
                        Use
                      </button>
                      <button type="button" onClick={() => onDelete(model.id)} disabled={busy}>
                        Delete
                      </button>
                    </>
                  ) : (
                    <button type="button" onClick={() => onDownload(model.id)} disabled={busy}>
                      Download
                    </button>
                  )}

                  {model.custom && model.installed ? (
                    <>
                      <button type="button" onClick={() => onUse(model)}>
                        Use
                      </button>
                      <button type="button" onClick={() => onDelete(model.id)} disabled={busy}>
                        Clear
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
