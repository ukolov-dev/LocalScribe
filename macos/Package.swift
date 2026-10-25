// swift-tools-version: 5.9
import PackageDescription
let package = Package(name: "LocalScribe", platforms: [.macOS(.v13)], products: [.executable(name: "LocalScribe", targets: ["LocalScribe"])], targets: [.executableTarget(name: "LocalScribe"), .testTarget(name: "LocalScribeTests", dependencies: ["LocalScribe"])])
