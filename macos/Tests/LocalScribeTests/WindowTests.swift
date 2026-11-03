import AppKit
import XCTest
@testable import LocalScribe

final class WindowTests: XCTestCase {
    @MainActor func testReopeningShowsTheExistingPanelWithoutTogglingItHidden() async throws {
        let app = NSApplication.shared
        let existingWindows = Set(app.windows.map(ObjectIdentifier.init))
        let delegate = AppDelegate()
        let panel = try XCTUnwrap(app.windows.first {
            $0 is NSPanel && !existingWindows.contains(ObjectIdentifier($0))
        })
        defer {
            if let item = delegate.statusItem { NSStatusBar.system.removeStatusItem(item) }
            panel.orderOut(nil); panel.close()
        }

        XCTAssertFalse(panel.isVisible, "Initial launch keeps the panel hidden")
        _ = delegate.applicationShouldHandleReopen(app, hasVisibleWindows: false)
        XCTAssertTrue(panel.isVisible, "Reopening must show the existing panel")
        _ = delegate.applicationShouldHandleReopen(app, hasVisibleWindows: true)
        XCTAssertTrue(panel.isVisible, "Repeated reopen must not hide the panel")

        panel.orderOut(nil)
        _ = delegate.applicationShouldHandleReopen(app, hasVisibleWindows: false)
        XCTAssertTrue(panel.isVisible, "Reopening must recover a hidden panel again")
    }
}
