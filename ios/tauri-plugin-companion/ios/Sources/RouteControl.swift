import AVKit
import UIKit
import WebKit

@MainActor
final class RouteControl: NSObject, AVRoutePickerViewDelegate {
    weak var webView: WKWebView?
    var presentationChanged: ((Bool) -> Void)?
    private var picker: AVRoutePickerView?
    private var presented = false
    private var pendingFrame: CGRect?

    func place(_ frame: CGRect?) {
        pendingFrame = frame
        guard !presented else { return }
        guard let frame, !frame.isEmpty, let webView, let parent = webView.superview else {
            picker?.isHidden = true
            return
        }
        let view: AVRoutePickerView
        if let existing = picker {
            view = existing
        } else {
            view = AVRoutePickerView()
            view.prioritizesVideoDevices = false
            view.delegate = self
            view.accessibilityLabel = "Audio output"
            picker = view
        }
        if view.superview !== parent { parent.insertSubview(view, aboveSubview: webView) }
        view.frame = webView.convert(frame, to: parent)
        view.isHidden = false
    }

    func routePickerViewWillBeginPresentingRoutes(_ routePickerView: AVRoutePickerView) {
        presented = true
        presentationChanged?(true)
    }

    func routePickerViewDidEndPresentingRoutes(_ routePickerView: AVRoutePickerView) {
        presented = false
        presentationChanged?(false)
        place(pendingFrame)
    }
}
