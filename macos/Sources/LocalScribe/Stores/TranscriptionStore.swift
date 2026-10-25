import AppKit
import SwiftUI
import UniformTypeIdentifiers

@MainActor
final class TranscriptionStore: ObservableObject {
    @Published var models: [SpeechModel] = []
    @Published var speakerModels: [SpeakerModel] = []
    @Published var modelID = UserDefaults.standard.string(forKey: "speechModel") ?? "" { didSet { UserDefaults.standard.set(modelID, forKey: "speechModel"); resumePending() } }
    @Published var language = UserDefaults.standard.string(forKey: "language") ?? "multi" { didSet { UserDefaults.standard.set(language, forKey: "language") } }
    @Published var preset = UserDefaults.standard.string(forKey: "preset") ?? "balanced" { didSet { UserDefaults.standard.set(preset, forKey: "preset") } }
    @Published var timestamps = UserDefaults.standard.object(forKey: "timestamps") as? Bool ?? true { didSet { UserDefaults.standard.set(timestamps, forKey: "timestamps") } }
    @Published var engineMode = UserDefaults.standard.string(forKey: "engineMode") ?? "auto" { didSet { UserDefaults.standard.set(engineMode, forKey: "engineMode") } }
    @Published var prompt = ""
    @Published var threads = 0
    @Published var text = "" { didSet { scheduleDraftSave() } }
    @Published var fileName = ""
    @Published var status = "Запуск движка…"
    @Published var progress = 0.0
    @Published var busy = false
    @Published var ready = false
    @Published var error: String?
    @Published var downloads: [String: Double] = [:]
    @Published var showModels = false
    @Published var isPresentingDialog = false
    var selectedModel: SpeechModel? { models.first { $0.id == modelID && $0.installed } }
    private let engine = EngineClient()
    private var currentJob: String?
    private var transcript: Transcript?
    private var pendingURL: URL?
    private var checkingImport = false
    private var polling: Task<Void, Never>?
    private var draftSave: Task<Void, Never>?

    func start() {
        if let data = try? Data(contentsOf: TranscriptDraft.url), let draft = try? JSONDecoder().decode(TranscriptDraft.self, from: data) {
            text = draft.text; fileName = draft.fileName; transcript = draft.transcript
        }
        engine.onEvent = { [weak self] name, payload in self?.event(name, payload) }
        engine.onExit = { [weak self] message in self?.ready = false; self?.busy = false; self?.error = message }
        do { try engine.start() } catch { self.error = error.localizedDescription; return }
        Task { [self] in
            do {
                struct Health: Decodable { let status: String }
                let _: Health = try await engine.call("health")
                try await refreshModels(); ready = true; status = "Готово"
                resumePending()
                polling = Task { [weak self] in
                    while !Task.isCancelled {
                        await self?.checkImports()
                        try? await Task.sleep(nanoseconds: 1_000_000_000)
                    }
                }
            } catch { self.error = error.localizedDescription }
        }
    }
    func stop() { polling?.cancel(); draftSave?.cancel(); saveDraft(); engine.stop() }
    private func scheduleDraftSave() {
        guard !busy else { return }
        draftSave?.cancel()
        draftSave = Task { [weak self] in
            do { try await Task.sleep(nanoseconds: 500_000_000) } catch { return }
            self?.saveDraft()
        }
    }
    private func saveDraft() {
        guard !text.isEmpty || !fileName.isEmpty else { return }
        do {
            let url = TranscriptDraft.url
            try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try JSONEncoder().encode(TranscriptDraft(text: text, fileName: fileName, transcript: transcript)).write(to: url, options: .atomic)
            try FileManager.default.setAttributes([.posixPermissions: 0o600], ofItemAtPath: url.path)
        } catch { self.error = "Не удалось сохранить черновик: \(error.localizedDescription)" }
    }
    func refreshModels() async throws {
        models = try await engine.call("list_models")
        speakerModels = try await engine.call("list_speaker_models")
        if selectedModel == nil { modelID = models.first(where: { $0.installed && $0.recommended })?.id ?? models.first(where: \.installed)?.id ?? "" }
    }
    func accept(_ urls: [URL]) {
        guard !busy, pendingURL == nil else { error = "Сначала дождитесь окончания текущей расшифровки."; return }
        do {
            pendingURL = try RecordingInput.validate(urls)
            error = nil
            if ready { resumePending() }
        } catch { self.error = error.localizedDescription }
    }
    private func resumePending() {
        guard ready, !busy, !checkingImport, let url = pendingURL else { return }
        guard selectedModel != nil else { showModels = true; status = "Выберите или загрузите модель — запись начнёт обрабатываться автоматически."; return }
        pendingURL = nil
        busy = true
        Task { await transcribe(url, imported: nil) }
    }
    func chooseFile() {
        isPresentingDialog = true; defer { isPresentingDialog = false }
        let panel = NSOpenPanel(); panel.allowsMultipleSelection = false; panel.canChooseDirectories = false
        panel.allowedContentTypes = RecordingInput.extensions.compactMap { UTType(filenameExtension: $0) }
        if panel.runModal() == .OK, let url = panel.url { accept([url]) }
    }
    private func transcribe(_ url: URL, imported: ImportRequest?) async {
        guard currentJob == nil, let selectedModel else { busy = false; return }
        busy = true; error = nil; fileName = imported?.originalFileName ?? url.lastPathComponent
        text = ""; transcript = nil; progress = 0; status = "Подготовка записи…"
        let job = imported?.id ?? UUID().uuidString; currentJob = job
        defer { busy = false; currentJob = nil; resumePending() }
        do {
            let options = TranscriptionOptions(jobId: job, filePath: url.path, modelId: selectedModel.id, language: imported?.options.language ?? language, decodingPreset: imported?.options.decodingPreset ?? preset, initialPrompt: prompt.isEmpty ? nil : prompt, timestamps: imported?.options.timestamps ?? timestamps, threads: threads > 0 ? threads : nil, engine: engineMode)
            let result: Transcript = try await engine.call("transcribe_file", ["options": options.json])
            transcript = result; text = result.text; progress = 100; saveDraft()
            status = result.benchmark.map { String(format: "Готово за %.1f с", Double($0.totalMs) / 1000) } ?? "Готово"
            if let imported {
                try await engine.callVoid("complete_integration_request", ["jobId": imported.id, "result": try JSONSerialization.jsonObject(with: JSONEncoder().encode(result))])
            }
        } catch {
            self.error = error.localizedDescription; status = "Расшифровка остановлена"
            if let imported { try? await engine.callVoid("fail_integration_request", ["jobId": imported.id, "message": error.localizedDescription]) }
        }
    }
    func cancel() { guard let job = currentJob else { return }; Task { try? await engine.callVoid("cancel_transcription", ["jobId": job]) } }
    private func checkImports() async {
        guard ready, !busy, pendingURL == nil, !checkingImport, selectedModel != nil else { return }
        checkingImport = true
        defer { checkingImport = false; resumePending() }
        do {
            let request: ImportRequest? = try await engine.call("claim_integration_request")
            if let request { await transcribe(URL(fileURLWithPath: request.mediaPath), imported: request) }
        } catch { self.error = error.localizedDescription }
    }
    func download(_ id: String, speakers: Bool = false) {
        let key = speakers ? "speaker:\(id)" : id
        guard downloads[key] == nil else { return }; downloads[key] = 0
        Task {
            defer { downloads[key] = nil }
            do {
                try await engine.callVoid(speakers ? "download_speaker_model" : "download_model", ["modelId": id])
                try await refreshModels()
                if !speakers { modelID = id }
                resumePending()
            } catch { self.error = error.localizedDescription }
        }
    }
    func delete(_ id: String, speakers: Bool = false) {
        Task {
            do { try await engine.callVoid(speakers ? "delete_speaker_model" : "delete_model", ["modelId": id]); try await refreshModels() }
            catch { self.error = error.localizedDescription }
        }
    }
    func customModel(speakers: Bool = false) {
        isPresentingDialog = true; defer { isPresentingDialog = false }
        let panel = NSOpenPanel(); panel.canChooseDirectories = false
        panel.allowedContentTypes = [UTType(filenameExtension: speakers ? "onnx" : "bin") ?? .data]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task {
            do { try await engine.callVoid(speakers ? "select_custom_speaker_model" : "select_custom_model", ["path": url.path]); try await refreshModels(); if !speakers { modelID = "custom" }; resumePending() }
            catch { self.error = error.localizedDescription }
        }
    }
    func copy() {
        NSPasteboard.general.clearContents()
        if NSPasteboard.general.setString(text, forType: .string) { status = "Скопировано" } else { error = "Не удалось скопировать текст." }
    }
    func export(_ format: String) {
        isPresentingDialog = true; defer { isPresentingDialog = false }
        let panel = NSSavePanel(); panel.nameFieldStringValue = (fileName as NSString).deletingPathExtension + "." + format
        panel.allowedContentTypes = [UTType(filenameExtension: format) ?? .plainText]
        guard panel.runModal() == .OK, let url = panel.url else { return }
        Task {
            do {
                let segments = text == transcript?.text ? transcript?.segments ?? [] : []
                let request: [String: Any] = ["format": format, "outputPath": url.path, "text": text, "segments": try JSONSerialization.jsonObject(with: JSONEncoder().encode(segments))]
                try await engine.callVoid("export_transcript", ["request": request]); status = "Сохранено"
            } catch { self.error = error.localizedDescription }
        }
    }
    private func event(_ name: String, _ payload: [String: Any]) {
        if name == "transcription-progress", payload["jobId"] as? String == currentJob {
            progress = payload["progress"] as? Double ?? progress
            switch payload["phase"] as? String {
            case "downloading-speaker-models": status = "Загрузка моделей спикеров…"
            case "assigning-speakers": status = "Определение спикеров…"
            case "converting-audio": status = "Подготовка аудио…"
            case "loading-model": status = "Загрузка модели…"
            default: status = "Расшифровка…"
            }
        } else if name == "transcription-segment", payload["jobId"] as? String == currentJob,
                  let value = payload["segment"], let data = try? JSONSerialization.data(withJSONObject: value), let segment = try? JSONDecoder().decode(Segment.self, from: data) {
            text += (text.isEmpty ? "" : "\n") + segment.displayText
        } else if name.hasSuffix("download-progress"), let id = payload["modelId"] as? String {
            let key = name.hasPrefix("speaker-") ? "speaker:\(id)" : id
            if downloads[key] != nil { downloads[key] = payload["progress"] as? Double ?? 0 }
            if busy, id == "speaker-neural", status.hasPrefix("Загрузка моделей спикеров") {
                status = "Загрузка моделей спикеров: \(Int(payload["progress"] as? Double ?? 0))%"
            }
        }
    }
}
