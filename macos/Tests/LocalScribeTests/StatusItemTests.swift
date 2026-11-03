import AppKit
import XCTest
@testable import LocalScribe

final class StatusItemTests: XCTestCase {
    @MainActor func testStatusItemStaysCompactAndCanBeRestoredWithoutDuplication() async throws {
        let app = NSApplication.shared
        let existingWindows = Set(app.windows.map(ObjectIdentifier.init))
        let delegate = AppDelegate()
        let previousDelegate = app.delegate
        app.delegate = delegate
        let panel = try XCTUnwrap(app.windows.first {
            $0 is NSPanel && !existingWindows.contains(ObjectIdentifier($0))
        })
        defer {
            if let item = delegate.statusItem { NSStatusBar.system.removeStatusItem(item) }
            panel.orderOut(nil)
            panel.close()
            app.delegate = previousDelegate
        }

        delegate.ensureStatusItem()
        let item = try XCTUnwrap(delegate.statusItem)
        let button = try XCTUnwrap(item.button)
        XCTAssertEqual(item.length, NSStatusItem.squareLength, "The status item must occupy one icon slot")
        XCTAssertEqual(button.title, "", "The status item must not consume space for a text title")
        XCTAssertNotNil(button.image)
        XCTAssertEqual(button.accessibilityLabel(), "LocalScribe")

        delegate.ensureStatusItem()
        XCTAssertTrue(delegate.statusItem === item, "Repeated setup must reuse the existing status item")
        item.isVisible = false
        delegate.ensureStatusItem()
        XCTAssertTrue(delegate.statusItem === item, "Restoring visibility must not create a duplicate")
        XCTAssertTrue(item.isVisible, "Setup must restore a hidden status item")

        item.isVisible = false
        delegate.applicationDidChangeScreenParameters(Notification(name: NSApplication.didChangeScreenParametersNotification))
        XCTAssertTrue(item.isVisible, "Display changes must restore the status item")
        item.isVisible = false
        _ = delegate.applicationShouldHandleReopen(app, hasVisibleWindows: false)
        XCTAssertTrue(item.isVisible, "Reopening must restore the status item")
        XCTAssertTrue(delegate.statusItem === item)
    }
}
