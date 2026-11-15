import UIKit
import UniformTypeIdentifiers

@MainActor
final class CloudFolderPicker: NSObject, UIDocumentPickerDelegate, UIAdaptivePresentationControllerDelegate {
    private var continuation: CheckedContinuation<(Data, String)?, Error>?

    func choose() async throws -> (Data, String)? {
        guard continuation == nil,
              let window = UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene })
                  .flatMap(\.windows).first(where: \.isKeyWindow),
              var presenter = window.rootViewController else {
            throw CloudLibrary.Failure("The folder picker is unavailable. Try again in Settings.")
        }
        while let next = presenter.presentedViewController { presenter = next }
        return try await withCheckedThrowingContinuation { result in
            continuation = result
            let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.folder], asCopy: false)
            picker.delegate = self
            picker.presentationController?.delegate = self
            presenter.present(picker, animated: true)
        }
    }

    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        finish(Result {
            guard let url = urls.first, url.startAccessingSecurityScopedResource() else {
                throw CloudLibrary.Failure("musicOS could not open that folder.")
            }
            defer { url.stopAccessingSecurityScopedResource() }
            return (try url.bookmarkData(options: .minimalBookmark), url.lastPathComponent)
        })
    }

    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) { finish(.success(nil)) }
    func presentationControllerDidDismiss(_ presentationController: UIPresentationController) { finish(.success(nil)) }
    private func finish(_ result: Result<(Data, String)?, Error>) {
        let current = continuation
        continuation = nil
        current?.resume(with: result)
    }
}
