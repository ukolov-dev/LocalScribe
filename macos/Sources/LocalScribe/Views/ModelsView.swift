import SwiftUI

struct ModelsView: View {
    @ObservedObject var store: TranscriptionStore
    @Environment(\.dismiss) private var dismiss
    @State private var speakers = false
    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            HStack {
                Text("Модели LocalScribe").font(.title2.weight(.semibold))
                Spacer()
                Button("Готово") { dismiss() }.keyboardShortcut(.defaultAction)
            }
            Picker("Тип моделей", selection: $speakers) { Text("Распознавание речи").tag(false); Text("Спикеры").tag(true) }.pickerStyle(.segmented)
            if speakers { Text("Нейросетевая диаризация всегда включена. Число голосов определяется автоматически. Модели загрузятся при первой расшифровке; затем обработка работает локально.").font(.caption).foregroundStyle(.secondary) }
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 16) {
                    if speakers {
                        ForEach(store.speakerModels.filter { $0.id == "speaker-neural" }) { model in
                            row(id: model.id, name: model.name, detail: "\(model.sizeLabel) · \(model.runtimeLabel)", installed: model.installed)
                        }
                    } else {
                        ForEach(store.models) { model in
                            row(id: model.id, name: model.name, detail: "\(model.sizeLabel) · \(model.qualityLabel)", installed: model.installed)
                        }
                    }
                }
            }
            if let error = store.error { Text(error).foregroundStyle(.red).font(.callout).textSelection(.enabled) }
            if !speakers { Button("Выбрать свою модель .bin…") { store.customModel() }.disabled(store.busy) }
        }.padding(22).frame(width: 540, height: 480)
    }
    private func row(id: String, name: String, detail: String, installed: Bool) -> some View {
        let key = speakers ? "speaker:\(id)" : id
        return VStack(alignment: .leading, spacing: 6) {
            HStack {
                VStack(alignment: .leading, spacing: 4) { Text(name).font(.headline); Text(detail).font(.caption).foregroundStyle(.secondary) }
                Spacer()
                if let progress = store.downloads[key] { Text("\(Int(progress))%").monospacedDigit() }
                else if installed {
                    if !speakers { Button(store.modelID == id ? "Выбрана" : "Выбрать") { store.modelID = id }.disabled(store.modelID == id) }
                    Button(role: .destructive) { store.delete(id, speakers: speakers) } label: { Image(systemName: "trash") }.help("Удалить модель")
                } else { Button("Загрузить") { store.download(id, speakers: speakers) } }
            }.disabled(store.busy)
            if let progress = store.downloads[key] { ProgressView(value: progress, total: 100) }
            Divider()
        }
    }
}
