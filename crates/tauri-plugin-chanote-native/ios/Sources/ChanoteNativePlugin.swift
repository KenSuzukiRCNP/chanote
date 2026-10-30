// chanote native helpers for iOS. See ../../src/lib.rs.

import Foundation
import SwiftRs
import Tauri
import UIKit
import UniformTypeIdentifiers
import WebKit

struct RestoreArgs: Decodable {
  let bookmark: String
}

struct PathArgs: Decodable {
  let path: String
}

class ChanoteNativePlugin: Plugin, UIDocumentPickerDelegate {
  private var pendingPick: Invoke?
  /// URLs we started accessing; kept for the lifetime of the app.
  private var accessed: [URL] = []

  @objc public func pickFolder(_ invoke: Invoke) {
    DispatchQueue.main.async {
      let picker = UIDocumentPickerViewController(forOpeningContentTypes: [UTType.folder])
      picker.delegate = self
      picker.allowsMultipleSelection = false
      picker.modalPresentationStyle = .formSheet
      self.pendingPick = invoke
      self.manager.viewController?.present(picker, animated: true, completion: nil)
    }
  }

  public func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
    guard let invoke = pendingPick else { return }
    pendingPick = nil
    guard let url = urls.first else {
      invoke.resolve(["cancelled": true])
      return
    }
    guard url.startAccessingSecurityScopedResource() else {
      invoke.reject("chanote was not given access to that folder")
      return
    }
    accessed.append(url)
    do {
      let data = try url.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil)
      invoke.resolve(["path": url.path, "bookmark": data.base64EncodedString(), "cancelled": false])
    } catch {
      invoke.reject("could not remember the folder: \(error.localizedDescription)")
    }
  }

  public func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    pendingPick?.resolve(["cancelled": true])
    pendingPick = nil
  }

  @objc public func restoreAccess(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(RestoreArgs.self)
    guard let data = Data(base64Encoded: args.bookmark) else {
      invoke.reject("invalid folder bookmark")
      return
    }
    var stale = false
    do {
      let url = try URL(resolvingBookmarkData: data, options: [], relativeTo: nil, bookmarkDataIsStale: &stale)
      guard url.startAccessingSecurityScopedResource() else {
        invoke.reject("access to the sync folder was refused; choose it again in Settings")
        return
      }
      accessed.append(url)
      var bookmark = args.bookmark
      if stale, let fresh = try? url.bookmarkData(options: [], includingResourceValuesForKeys: nil, relativeTo: nil) {
        bookmark = fresh.base64EncodedString()
      }
      invoke.resolve(["path": url.path, "bookmark": bookmark, "stale": stale, "cancelled": false])
    } catch {
      invoke.reject("the sync folder could not be found: \(error.localizedDescription)")
    }
  }

  /// Start downloading every cloud-only file below `path`. Handles both the classic
  /// ".name.icloud" placeholders and dataless files reported through resource values.
  @objc public func ensureDownloaded(_ invoke: Invoke) throws {
    let args = try invoke.parseArgs(PathArgs.self)
    let root = URL(fileURLWithPath: args.path)
    let fm = FileManager.default
    var pending = 0
    let keys: [URLResourceKey] = [.ubiquitousItemDownloadingStatusKey, .isUbiquitousItemKey]
    if let en = fm.enumerator(at: root, includingPropertiesForKeys: keys, options: [], errorHandler: nil) {
      for case let url as URL in en {
        let name = url.lastPathComponent
        if name.hasPrefix(".") && name.hasSuffix(".icloud") {
          let realName = String(name.dropFirst().dropLast(".icloud".count))
          let real = url.deletingLastPathComponent().appendingPathComponent(realName)
          try? fm.startDownloadingUbiquitousItem(at: real)
          pending += 1
          continue
        }
        if let v = try? url.resourceValues(forKeys: Set(keys)),
          v.isUbiquitousItem == true,
          let status = v.ubiquitousItemDownloadingStatus,
          status == .notDownloaded
        {
          try? fm.startDownloadingUbiquitousItem(at: url)
          pending += 1
        }
      }
    }
    invoke.resolve(["pending": pending])
  }
}

@_cdecl("init_plugin_chanote_native")
public func initPlugin() -> Plugin {
  return ChanoteNativePlugin()
}
