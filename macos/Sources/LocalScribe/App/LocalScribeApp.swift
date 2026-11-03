import AppKit
import OSLog
import SwiftUI

@main
enum LocalScribeMain {
    @MainActor static func main() {
        let app = NSApplication.shared
        let delegate = AppDelegate()
        app.delegate = delegate
        app.setActivationPolicy(.accessory)
        withExtendedLifetime(delegate) { app.run() }
    }
}

private final class TrayPanel: NSPanel {
    override var canBecomeKey: Bool { true }
    override var canBecomeMain: Bool { false }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let store = TranscriptionStore()
    private let panel = TrayPanel(contentRect: NSRect(x: 0, y: 0, width: 460, height: 680), styleMask: [.borderless, .nonactivatingPanel], backing: .buffered, defer: false)
    private(set) var statusItem: NSStatusItem!
    private var outsideMonitor: Any?
    private let menuBarLogger = Logger(subsystem: "com.localscribe.menubar", category: "MenuBar")

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)
        panel.isFloatingPanel = true; panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false; panel.level = .floating
        panel.backgroundColor = .clear; panel.isOpaque = false; panel.hasShadow = true
        panel.title = "LocalScribe"
        let content = ContentView(store: store, close: { [weak self] in self?.panel.orderOut(nil) })
            .background(.regularMaterial).clipShape(RoundedRectangle(cornerRadius: 14))
        panel.contentView = NSHostingView(rootView: content)
        panel.setContentSize(NSSize(width: 460, height: 680))
        ensureStatusItem()
        // Mouse-down in Finder starts a drag. Dismiss only when an outside click finishes.
        outsideMonitor = NSEvent.addGlobalMonitorForEvents(matching: [.leftMouseUp, .rightMouseUp]) { [weak self] _ in
            Task { @MainActor in
                guard let self, self.panel.isVisible, !self.store.isPresentingDialog, !self.store.showModels,
                      !self.statusFrame().contains(NSEvent.mouseLocation), !self.panel.frame.contains(NSEvent.mouseLocation) else { return }
                self.panel.orderOut(nil)
            }
        }
        store.start()
        DispatchQueue.main.async { self.logStatusItem("launch") }
    }
    func ensureStatusItem() {
        if statusItem == nil {
            statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
            if let button = statusItem.button {
                button.image = NSImage(systemSymbolName: "waveform", accessibilityDescription: "LocalScribe")
                button.toolTip = "LocalScribe — открыть расшифровку"
                button.setAccessibilityLabel("LocalScribe")
                button.target = self; button.action = #selector(trayClicked)
                button.sendAction(on: [.leftMouseUp, .rightMouseUp])
            }
        }
        statusItem.isVisible = true
    }
    private func logStatusItem(_ event: String) {
        guard let item = statusItem else { return }
        let window = item.button?.window
        let frame = window?.frame ?? .zero
        let screen = window?.screen ?? NSScreen.main
        menuBarLogger.debug("event=\(event, privacy: .public) visible=\(item.isVisible) windowVisible=\(window?.isVisible ?? false) occluded=\(!(window?.occlusionState.contains(.visible) ?? false)) frame=\(NSStringFromRect(frame), privacy: .public) screen=\(NSStringFromRect(screen?.frame ?? .zero), privacy: .public) rightArea=\(NSStringFromRect(screen?.auxiliaryTopRightArea ?? .zero), privacy: .public)")
    }
    func applicationDidBecomeActive(_ notification: Notification) {
        ensureStatusItem()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { self.logStatusItem("active") }
    }
    func applicationDidChangeScreenParameters(_ notification: Notification) {
        ensureStatusItem()
        DispatchQueue.main.async { self.logStatusItem("screensChanged") }
    }
    func applicationDidResignActive(_ notification: Notification) {
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.3) { self.logStatusItem("inactive") }
    }
    private func activeScreen() -> NSScreen? { NSScreen.screens.first { $0.frame.contains(NSEvent.mouseLocation) } ?? NSScreen.main }
    private func statusFrame() -> NSRect {
        guard let button = statusItem?.button, let window = button.window, let source = window.screen, let target = activeScreen() else { return .zero }
        // Status items have mirrored copies; Accessibility can address the primary copy.
        return window.convertToScreen(button.convert(button.bounds, to: nil))
            .offsetBy(dx: target.frame.maxX - source.frame.maxX, dy: target.frame.maxY - source.frame.maxY)
    }
    @objc private func trayClicked() {
        let event = NSApp.currentEvent
        let fresh = event.map { ProcessInfo.processInfo.systemUptime - $0.timestamp < 0.5 } ?? false
        if fresh && event?.type == .rightMouseUp {
            let menu = NSMenu()
            menu.addItem(withTitle: "Показать / скрыть", action: #selector(togglePanel), keyEquivalent: "").target = self
            menu.addItem(.separator())
            menu.addItem(withTitle: "Выйти", action: #selector(quit), keyEquivalent: "q").target = self
            menu.popUp(positioning: nil, at: NSEvent.mouseLocation, in: nil)
        } else { togglePanel() }
    }
    @objc private func togglePanel() {
        if panel.isVisible { panel.orderOut(nil); return }
        showPanel()
    }
    private func showPanel() {
        if let screen = activeScreen() {
            let icon = statusFrame(), area = screen.visibleFrame, size = panel.frame.size
            let x = min(max(icon.midX - size.width / 2, area.minX), max(area.minX, area.maxX - size.width))
            let y = min(max(icon.minY - size.height - 8, area.minY), max(area.minY, area.maxY - size.height))
            panel.setFrameOrigin(NSPoint(x: x, y: y))
        }
        NSApp.activate(ignoringOtherApps: true)
        panel.makeKeyAndOrderFront(nil)
    }
    @objc private func quit() { NSApp.terminate(nil) }
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        ensureStatusItem()
        showPanel()
        return false
    }
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard store.busy else { return .terminateNow }
        let alert = NSAlert(); alert.messageText = "Остановить расшифровку и выйти?"
        alert.informativeText = "Текущая расшифровка ещё не завершена."
        alert.addButton(withTitle: "Продолжить расшифровку"); alert.addButton(withTitle: "Остановить и выйти")
        return alert.runModal() == .alertSecondButtonReturn ? .terminateNow : .terminateCancel
    }
    func applicationWillTerminate(_ notification: Notification) {
        if let outsideMonitor { NSEvent.removeMonitor(outsideMonitor) }
        store.stop()
    }
}
