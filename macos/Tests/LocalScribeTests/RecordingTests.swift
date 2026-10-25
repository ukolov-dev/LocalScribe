import XCTest
@testable import LocalScribe

final class RecordingTests: XCTestCase {
    func testRejectsMultipleFilesAndNonAudioWithoutReadingThem() throws {
        let a = URL(fileURLWithPath: "/tmp/a.wav")
        XCTAssertThrowsError(try RecordingInput.validate([a, a]))
        XCTAssertThrowsError(try RecordingInput.validate([URL(string: "https://example.org/a.wav")!]))
        XCTAssertThrowsError(try RecordingInput.validate([URL(fileURLWithPath: "/tmp/a.txt")]))
    }
    func testAcceptsAnExistingRecordingAndRejectsDirectoriesAndMissingFiles() throws {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString + ".WAV")
        defer { try? FileManager.default.removeItem(at: url) }
        try Data([1]).write(to: url)
        XCTAssertEqual(try RecordingInput.validate([url]), url.standardizedFileURL)
        try FileManager.default.removeItem(at: url)
        XCTAssertThrowsError(try RecordingInput.validate([url]))
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: false)
        XCTAssertThrowsError(try RecordingInput.validate([url]))
    }
    func testEveryNativeRequestIncludesSpeakers() {
        let options = TranscriptionOptions(jobId: "job", filePath: "/tmp/a.wav", modelId: "large-v3-turbo", language: "multi", decodingPreset: "balanced", initialPrompt: nil, timestamps: true, threads: nil)
        XCTAssertEqual(options.json["speakerMode"] as? String, "auto")
        XCTAssertNil(options.json["threads"])
    }
    func testIntegrationResultPreservesSpeakerLabelsAndBenchmarkPhases() throws {
        let data = Data(#"{"jobId":"job","text":"Speaker 1: Hello","segments":[{"index":0,"startMs":0,"endMs":1000,"text":"Hello","speaker":"Speaker 1"}],"benchmark":{"totalMs":123,"audioDurationMs":1000,"phases":[{"phase":"decode","durationMs":100}]}}"#.utf8)
        let result = try JSONDecoder().decode(Transcript.self, from: data)
        let roundTrip = try JSONDecoder().decode(Transcript.self, from: JSONEncoder().encode(result))
        XCTAssertEqual(roundTrip.segments.first?.displayText, "Speaker 1: Hello")
        XCTAssertEqual(roundTrip.benchmark?.phases.first?.durationMs, 100)
    }
    @MainActor func testTransportHandlesPartialUnicodeAndMultipleEvents() async {
        let client = EngineClient()
        var texts: [String] = []
        client.onEvent = { _, payload in texts.append(payload["text"] as? String ?? "") }
        let lines = "noise\n{\"event\":\"segment\",\"payload\":{\"text\":\"Привет\"}}\n{\"event\":\"segment\",\"payload\":{\"text\":\"Hello\"}}\n"
        let bytes = Data(lines.utf8)
        for byte in bytes.dropLast() { client.receiveChunk(Data([byte])) }
        XCTAssertEqual(texts, ["Привет"])
        client.receiveChunk(Data([10]))
        XCTAssertEqual(texts, ["Привет", "Hello"])
    }

}
