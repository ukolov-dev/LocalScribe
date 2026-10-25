import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @ObservedObject var store: TranscriptionStore
    let close: () -> Void
    @State private var settingsOpen = false
    @State private var targeted = false

    var body: some View {
        VStack(spacing: 16) {
            HStack(spacing: 9) {
                Image(systemName: "waveform").foregroundStyle(Color.accentColor).font(.title2)
                Text("LocalScribe").font(.headline)
                Spacer()
                if store.busy { ProgressView().controlSize(.small) }
                Button(action: close) { Image(systemName: "xmark") }.buttonStyle(.plain).help("Скрыть панель")
            }
            Button(action: store.chooseFile) {
                VStack(spacing: 10) {
                    Image(systemName: targeted ? "arrow.down.doc.fill" : "arrow.up.doc").font(.system(size: 32, weight: .light))
                    Text(store.busy ? "Обрабатывается запись…" : "Перетащите запись").font(.title3.weight(.semibold))
                    Text("Расшифровка начнётся автоматически").font(.callout)
                    Text("AAC · MP3 · WAV · M4A · MP4 · MOV · WebM").font(.caption).foregroundStyle(.secondary)
                }
                .frame(maxWidth: .infinity).padding(.vertical, 23)
                .background(Color.accentColor.opacity(targeted ? 0.17 : 0.07), in: RoundedRectangle(cornerRadius: 14))
                .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Color.accentColor.opacity(targeted ? 0.9 : 0.35), style: StrokeStyle(lineWidth: 1, dash: [5])))
                .contentShape(RoundedRectangle(cornerRadius: 14))
            }
            .buttonStyle(.plain)
            .onDrop(of: [UTType.fileURL.identifier], isTargeted: $targeted, perform: dropped)
            HStack(spacing: 6) {
                Image(systemName: "person.2.fill").foregroundStyle(Color.accentColor)
                Text("Спикеры определяются всегда").font(.callout)
                Spacer()
            }
            ScrollView {
                DisclosureGroup("Настройки", isExpanded: $settingsOpen) {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Picker("Модель", selection: $store.modelID) {
                                Text("Выберите модель").tag("")
                                ForEach(store.models.filter(\.installed)) { Text($0.name).tag($0.id) }
                            }.disabled(store.busy)
                            Button("Модели…") { store.showModels = true }
                        }
                        Picker("Вычисления", selection: $store.engineMode) {
                            Text("Автоматически").tag("auto")
                            Text("GPU / Metal").tag("metal")
                            Text("CPU").tag("cpu")
                        }
                        Picker("Язык записи", selection: $store.language) {
                            Text("Русский + English").tag("multi")
                            Text("Определять автоматически").tag("auto")
                            Text("Русский").tag("ru")
                            Text("English").tag("en")
                            Text("Многоязычный — быстро").tag("multi-fast")
                            Text("Deutsch").tag("de")
                            Text("Français").tag("fr")
                            Text("Español").tag("es")
                            Text("Italiano").tag("it")
                            Text("Português").tag("pt")
                            Text("中文").tag("zh")
                            Text("日本語").tag("ja")
                        }
                        Picker("Распознавание", selection: $store.preset) {
                            Text("Сбалансированное").tag("balanced")
                            Text("Точное").tag("accurate")
                        }
                        Toggle("Таймкоды", isOn: $store.timestamps)
                        TextField("Контекст и термины (необязательно)", text: $store.prompt)
                        Stepper(store.threads == 0 ? "Потоки: автоматически" : "Потоки: \(store.threads)", value: $store.threads, in: 0...max(1, ProcessInfo.processInfo.processorCount))
                        Text("Нейросеть определяет число голосов автоматически. При шуме и одновременной речи возможны ошибки.").font(.caption).foregroundStyle(.secondary)
                    }.padding(.top, 10).disabled(store.busy)
                }.padding(12).background(.quaternary.opacity(0.4), in: RoundedRectangle(cornerRadius: 10))
            }.frame(maxHeight: settingsOpen ? 245 : 44)
            if let error = store.error {
                HStack(alignment: .top) {
                    Image(systemName: "exclamationmark.triangle.fill")
                    Text(error).textSelection(.enabled)
                    Spacer(minLength: 0)
                    Button { store.error = nil } label: { Image(systemName: "xmark") }.buttonStyle(.plain)
                }.font(.callout).foregroundStyle(.red).fixedSize(horizontal: false, vertical: true)
            }
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Транскрипт").font(.headline)
                    Spacer()
                    Button(action: store.copy) { Image(systemName: "doc.on.doc") }.help("Скопировать текст")
                    Button("TXT") { store.export("txt") }
                    Button("MD") { store.export("md") }
                }.disabled(store.text.isEmpty || store.busy)
                TextEditor(text: $store.text)
                    .font(.system(size: 13)).scrollContentBackground(.hidden)
                    .padding(6).background(.background.opacity(0.5), in: RoundedRectangle(cornerRadius: 9))
                    .overlay(RoundedRectangle(cornerRadius: 9).strokeBorder(.quaternary))
                    .frame(minHeight: 100, maxHeight: .infinity).disabled(store.busy)
                    .accessibilityLabel("Текст расшифровки со спикерами")
            }
            VStack(spacing: 8) {
                if store.busy { ProgressView(value: store.progress, total: 100) }
                HStack {
                    VStack(alignment: .leading, spacing: 3) {
                        Text(store.status).font(.caption).foregroundStyle(.secondary)
                        if !store.fileName.isEmpty { Text(store.fileName).font(.caption2).lineLimit(1).truncationMode(.middle).foregroundStyle(.secondary) }
                    }
                    Spacer()
                    if store.busy { Button("Остановить", action: store.cancel) }
                    Menu {
                        Button("Выбрать запись…", action: store.chooseFile).disabled(store.busy)
                        Button("Модели…") { store.showModels = true }
                        Divider()
                        Button("Выйти из LocalScribe") { NSApplication.shared.terminate(nil) }
                    } label: { Image(systemName: "ellipsis.circle") }.menuStyle(.borderlessButton).fixedSize()
                }
            }
        }
        .padding(18).frame(width: 460, height: 680)
        .tint(Color(red: 0.17, green: 0.45, blue: 0.36))
        .sheet(isPresented: $store.showModels) { ModelsView(store: store) }
        .onExitCommand(perform: close)
    }
    private func dropped(_ providers: [NSItemProvider]) -> Bool {
        guard providers.count == 1, let provider = providers.first else { store.error = "Перетащите одну запись."; return false }
        guard !store.busy else { store.error = "Сначала дождитесь окончания текущей расшифровки."; return false }
        provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { value, error in
            let url: URL?
            if let data = value as? Data { url = URL(dataRepresentation: data, relativeTo: nil) }
            else { url = value as? URL }
            Task { @MainActor in
                if let url { store.accept([url]) }
                else { store.error = error?.localizedDescription ?? "Не удалось прочитать перетянутый файл." }
            }
        }
        return true
    }
}
