import ActivityKit
import Foundation
@available(iOS 16.1,*)
public struct SyncActivityAttributes:ActivityAttributes {
    public struct ContentState:Codable,Hashable {
        public var phase:String
        public var message:String
        public var completed:Int
        public var total:Int
        public var jobID:String?
        public init(phase:String,message:String,completed:Int,total:Int,jobID:String?=nil){self.phase=phase;self.message=message;self.completed=completed;self.total=total;self.jobID=jobID}
        public var fraction:Double{total>0 ? min(1,max(0,Double(completed)/Double(total))):0}
    }
    public var computer:String
    public init(computer:String){self.computer=computer}
}
