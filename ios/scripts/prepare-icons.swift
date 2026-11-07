import Foundation
import CoreGraphics
import ImageIO
let source=URL(fileURLWithPath:CommandLine.arguments[1]),directory=URL(fileURLWithPath:CommandLine.arguments[2])
guard let input=CGImageSourceCreateWithURL(source as CFURL,nil),let image=CGImageSourceCreateImageAtIndex(input,0,nil) else {fatalError("Missing musicOS icon master")}
let manifest=try JSONSerialization.jsonObject(with:Data(contentsOf:directory.appendingPathComponent("Contents.json"))) as! [String:Any]
for item in manifest["images"] as! [[String:Any]] {
 guard let file=item["filename"] as? String,let size=item["size"] as? String,let scale=item["scale"] as? String,let side=Double(size.split(separator:"x")[0]),let factor=Double(scale.dropLast()) else {continue}
 let pixels=Int(side*factor),rect=CGRect(x:0,y:0,width:pixels,height:pixels)
 guard let context=CGContext(data:nil,width:pixels,height:pixels,bitsPerComponent:8,bytesPerRow:pixels*4,space:CGColorSpaceCreateDeviceRGB(),bitmapInfo:CGImageAlphaInfo.noneSkipLast.rawValue) else {fatalError("Could not prepare icon canvas")}
 context.setFillColor(CGColor(red:0.23,green:0.55,blue:0.88,alpha:1));context.fill(rect);context.interpolationQuality = .high;context.draw(image,in:rect)
 guard let output=context.makeImage(),let writer=CGImageDestinationCreateWithURL(directory.appendingPathComponent(file) as CFURL,"public.png" as CFString,1,nil) else {fatalError("Could not export icon")}
 CGImageDestinationAddImage(writer,output,nil);if !CGImageDestinationFinalize(writer){fatalError("Icon export failed")}
}
