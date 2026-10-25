import Foundation

@MainActor
final class EngineClient {
    var onEvent: ((String, [String: Any]) -> Void)?
    var onExit: ((String) -> Void)?
    private var process: Process?
    private var input: FileHandle?
    private var pending: [String: CheckedContinuation<Data, Error>] = [:]
    private var closing = false
    private var received = Data()

    func start() throws {
        guard process == nil else { return }
        let executable = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/localscribe-engine")
        guard FileManager.default.isExecutableFile(atPath: executable.path) else { throw NativeError("Движок LocalScribe отсутствует в приложении.") }
        let child = Process(), stdin = Pipe(), stdout = Pipe(), stderr = Pipe()
        child.executableURL = executable
        child.arguments = ["--native-worker"]
        child.standardInput = stdin; child.standardOutput = stdout; child.standardError = stderr
        // Whisper's diagnostics stay separate from the JSON transport.
        stderr.fileHandleForReading.readabilityHandler = { handle in _ = handle.availableData }
        child.terminationHandler = { [weak self] child in
            stdout.fileHandleForReading.readabilityHandler = nil
            stderr.fileHandleForReading.readabilityHandler = nil
            Task { @MainActor in self?.exited(status: child.terminationStatus) }
        }
        try child.run()
        input = stdin.fileHandleForWriting; process = child
        stdout.fileHandleForReading.readabilityHandler = { [weak self] handle in
            let chunk = handle.availableData
            if chunk.isEmpty { handle.readabilityHandler = nil; return }
            DispatchQueue.main.async { self?.receiveChunk(chunk) }
        }
    }
    func receiveChunk(_ chunk: Data) {
        received.append(chunk)
        while let newline = received.firstIndex(of: 10) {
            let line = Data(received[..<newline]); received.removeSubrange(...newline)
            receive(line)
        }
    }

    func call<T: Decodable>(_ command: String, _ args: [String: Any] = [:]) async throws -> T {
        let data = try await send(command, args)
        return try JSONDecoder().decode(T.self, from: data)
    }
    func callVoid(_ command: String, _ args: [String: Any] = [:]) async throws { _ = try await send(command, args) }
    private func send(_ command: String, _ args: [String: Any]) async throws -> Data {
        guard let process, process.isRunning, let input else { throw NativeError("Движок не запущен. Перезапустите LocalScribe.") }
        let id = UUID().uuidString
        var message = try JSONSerialization.data(withJSONObject: ["id": id, "command": command, "args": args])
        message.append(10)
        return try await withCheckedThrowingContinuation { continuation in
            pending[id] = continuation
            do { try input.write(contentsOf: message) }
            catch { pending.removeValue(forKey: id)?.resume(throwing: error) }
        }
    }
    private func receive(_ data: Data) {
        guard let message = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return }
        if let event = message["event"] as? String, let payload = message["payload"] as? [String: Any] { onEvent?(event, payload); return }
        guard let id = message["id"] as? String, let continuation = pending.removeValue(forKey: id) else { return }
        if let error = message["error"] as? [String: Any] { continuation.resume(throwing: NativeError(error["message"] as? String ?? "Ошибка движка.")) }
        else {
            do { continuation.resume(returning: try JSONSerialization.data(withJSONObject: message["result"] ?? NSNull(), options: [.fragmentsAllowed])) }
            catch { continuation.resume(throwing: error) }
        }
    }
    private func exited(status: Int32) {
        process = nil; input = nil
        let message = "Движок LocalScribe завершился (\(status)). Перезапустите приложение."
        let waiting = pending; pending.removeAll()
        for continuation in waiting.values { continuation.resume(throwing: NativeError(message)) }
        if !closing { onExit?(message) }
    }
    func stop() {
        closing = true
        try? input?.close(); input = nil
        // EOF lets the worker release the local API and active jobs on app exit.
        if let child = process, child.isRunning { child.terminate() }
    }
}
