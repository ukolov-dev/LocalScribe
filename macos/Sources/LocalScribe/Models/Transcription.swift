import Foundation

struct SpeechModel: Decodable, Identifiable {
    let id: String
    let name: String
    let sizeLabel: String
    let qualityLabel: String
    let recommended: Bool
    let installed: Bool
    let custom: Bool
}
struct SpeakerModel: Decodable, Identifiable {
    let id: String
    let name: String
    let sizeLabel: String
    let qualityLabel: String
    let runtimeLabel: String
    let installed: Bool
    let custom: Bool
}
struct Segment: Codable {
    let index: Int
    let startMs: UInt64
    let endMs: UInt64
    let text: String
    let speaker: String?
    var displayText: String { speaker.map { "\($0): \(text)" } ?? text }
}
struct Transcript: Codable {
    let jobId: String
    let text: String
    let segments: [Segment]
    let benchmark: Benchmark?
    struct Benchmark: Codable { let totalMs: UInt64; let audioDurationMs: UInt64; let phases: [Phase] }
    struct Phase: Codable { let phase: String; let durationMs: UInt64 }
}
struct ImportRequest: Decodable {
    let id: String
    let mediaPath: String
    let originalFileName: String
    let options: Options
    struct Options: Decodable { let language: String?; let decodingPreset: String?; let timestamps: Bool? }
}
struct TranscriptionOptions: Encodable {
    let jobId: String
    let filePath: String
    let modelId: String
    let language: String
    let decodingPreset: String
    let initialPrompt: String?
    let timestamps: Bool
    let threads: Int?
    // Speaker identification is part of every transcription, including imports.
    let speakerMode = "auto"
    var engine: String = "auto"
    var json: [String: Any] { (try? JSONSerialization.jsonObject(with: JSONEncoder().encode(self))) as? [String: Any] ?? [:] }
}
enum RecordingInput {
    static let extensions: Set<String> = ["aac", "mp3", "wav", "m4a", "mp4", "mov", "webm"]
    static func validate(_ urls: [URL]) throws -> URL {
        guard urls.count == 1 else { throw NativeError("Перетащите одну запись.") }
        let url = urls[0].standardizedFileURL
        guard url.isFileURL, extensions.contains(url.pathExtension.lowercased()) else { throw NativeError("Поддерживаются AAC, MP3, WAV, M4A, MP4, MOV и WebM.") }
        var directory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: url.path, isDirectory: &directory), !directory.boolValue else { throw NativeError("Файл записи не найден.") }
        return url
    }
}
struct NativeError: LocalizedError {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

struct TranscriptDraft: Codable {
    let text: String
    let fileName: String
    let transcript: Transcript?
    static var url: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("com.localscribe.desktop/native-draft.json")
    }
}
