import Foundation

final class MacDiscovery:NSObject,NetServiceBrowserDelegate,NetServiceDelegate {
    let browser=NetServiceBrowser();var services:[NetService]=[];var results:[[String:String]]=[]
    var finish:(([String:String])->Void)?
    func start(){stop();results=[];browser.delegate=self;browser.searchForServices(ofType:"_musicos-sync._tcp.",inDomain:"local.")}
    func stop(){browser.stop();services.forEach{$0.stop()};services=[]}
    func netServiceBrowser(_ browser:NetServiceBrowser,didFind service:NetService,moreComing:Bool){services.append(service);service.delegate=self;service.resolve(withTimeout:4)}
    func netServiceDidResolveAddress(_ sender:NetService){
        let txt=sender.txtRecordData().map{NetService.dictionary(fromTXTRecord:$0)} ?? [:]
        guard let id=txt["id"].flatMap({String(data:$0,encoding:.utf8)}),let fingerprint=txt["fingerprint"].flatMap({String(data:$0,encoding:.utf8)}),let host=sender.hostName else{return}
        let row=["id":id,"fingerprint":fingerprint,"host":host,"port":String(sender.port),"name":sender.name]
        results.removeAll{$0["id"]==id};results.append(row);finish?(row)
    }
}
