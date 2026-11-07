import UIKit
import AVFoundation

@MainActor func scanPairingCode() async throws -> [String:String] {
    let permission=AVCaptureDevice.authorizationStatus(for:.video)
    let allowed=permission == .authorized ? true : permission == .notDetermined ? await AVCaptureDevice.requestAccess(for:.video) : false
    guard allowed else {throw NSError(domain:"musicOS",code:1,userInfo:[NSLocalizedDescriptionKey:"Allow camera access in iPhone Settings to scan a code, or paste the pairing link instead."])}
    guard let controller=UIApplication.shared.connectedScenes.compactMap({$0 as? UIWindowScene}).flatMap(\.windows).first(where:{$0.isKeyWindow})?.rootViewController else {throw NSError(domain:"musicOS",code:2,userInfo:[NSLocalizedDescriptionKey:"Open musicOS before scanning."])}
    return try await withCheckedThrowingContinuation { continuation in
        let scanner=PairingScanner();scanner.modalPresentationStyle = .fullScreen
        scanner.finished={result in continuation.resume(with:result)}
        var presenter=controller;while let presented=presenter.presentedViewController{presenter=presented}
        presenter.present(scanner,animated:true)
    }
}
private final class PairingScanner:UIViewController,AVCaptureMetadataOutputObjectsDelegate {
    let session=AVCaptureSession(),worker=DispatchQueue(label:"musicOS.pairing-camera",qos:.userInitiated)
    var preview:AVCaptureVideoPreviewLayer?
    var finished:((Result<[String:String],Error>)->Void)?
    let guide=UIView(),hint=UILabel()
    override func viewDidLoad(){
        super.viewDidLoad();view.backgroundColor = .black
        let layer=AVCaptureVideoPreviewLayer(session:session);layer.videoGravity = .resizeAspectFill;view.layer.addSublayer(layer);preview=layer
        guide.layer.borderWidth=3;guide.layer.borderColor=UIColor(red:0.65,green:0.9,blue:0.95,alpha:1).cgColor;guide.layer.cornerRadius=28;guide.isUserInteractionEnabled=false;view.addSubview(guide)
        hint.text="Scan your Mac’s musicOS pairing code";hint.textColor = .white;hint.textAlignment = .center;hint.numberOfLines=0;hint.font=UIFont(name:"Figtree",size:20) ?? .preferredFont(forTextStyle:.headline);view.addSubview(hint)
        let close=UIButton(type:.system);close.setTitle("Cancel",for:.normal);close.tintColor = .white;close.backgroundColor=UIColor.white.withAlphaComponent(0.18);close.layer.cornerRadius=18;close.translatesAutoresizingMaskIntoConstraints=false;close.addTarget(self,action:#selector(cancel),for:.touchUpInside);view.addSubview(close)
        NSLayoutConstraint.activate([close.leadingAnchor.constraint(equalTo:view.safeAreaLayoutGuide.leadingAnchor,constant:20),close.topAnchor.constraint(equalTo:view.safeAreaLayoutGuide.topAnchor,constant:12),close.widthAnchor.constraint(equalToConstant:86),close.heightAnchor.constraint(equalToConstant:44)])
        worker.async { [weak self] in
            guard let self=self else{return}
            do{
                guard let camera=AVCaptureDevice.default(for:.video) else {throw NSError(domain:"musicOS",code:3,userInfo:[NSLocalizedDescriptionKey:"No camera is available. Paste the pairing link instead."])}
                let input=try AVCaptureDeviceInput(device:camera),output=AVCaptureMetadataOutput()
                self.session.beginConfiguration();self.session.sessionPreset = .high
                guard self.session.canAddInput(input),self.session.canAddOutput(output) else {self.session.commitConfiguration();throw NSError(domain:"musicOS",code:4,userInfo:[NSLocalizedDescriptionKey:"The camera is unavailable."])}
                self.session.addInput(input);self.session.addOutput(output);output.setMetadataObjectsDelegate(self,queue:.main);output.metadataObjectTypes=[.qr];self.session.commitConfiguration();self.session.startRunning()
            }catch{DispatchQueue.main.async{self.finish(.failure(error))}}
        }
    }
    override func viewDidLayoutSubviews(){super.viewDidLayoutSubviews();preview?.frame=view.bounds;let size=min(view.bounds.width-56,300);guide.frame=CGRect(x:(view.bounds.width-size)/2,y:(view.bounds.height-size)/2,width:size,height:size);hint.frame=CGRect(x:28,y:guide.frame.maxY+28,width:view.bounds.width-56,height:72)}
    override func viewDidDisappear(_ animated:Bool){super.viewDidDisappear(animated);worker.async{[session] in session.stopRunning()}}
    @objc func cancel(){finish(.success([:]))}
    func finish(_ result:Result<[String:String],Error>){guard let callback=finished else{return};finished=nil;worker.async{[session] in session.stopRunning()};dismiss(animated:true){callback(result)}}
    func metadataOutput(_ output:AVCaptureMetadataOutput,didOutput metadataObjects:[AVMetadataObject],from connection:AVCaptureConnection){
        guard let code=metadataObjects.compactMap({$0 as? AVMetadataMachineReadableCodeObject}).first?.stringValue else{return}
        guard let url=URLComponents(string:code),url.scheme=="musicos",url.host=="pair" else{hint.text="That isn’t a musicOS pairing code. Open Devices on your Mac.";return}
        UINotificationFeedbackGenerator().notificationOccurred(.success);finish(.success(["link":code]))
    }
}
