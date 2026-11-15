import ActivityKit
import WidgetKit
import SwiftUI
import MusicOSActivityShared

@main struct MusicOSSyncWidgets:WidgetBundle {var body:some Widget{MusicOSSyncWidget()}}
struct MusicOSSyncWidget:Widget {
    let blue=Color(red:0.58,green:0.82,blue:0.95)
    private func title(_ phase: String) -> String {
        switch phase {
        case "mac_job": return "Mac activity"
        case "complete": return "Ready offline"
        case "approval": return "Waiting for approval"
        case "error": return "Sync needs attention"
        case "cancelled", "cancelling": return "Sync stopped"
        case "preparing": return "Preparing music"
        case "updating": return "Updating library"
        case "uploading": return "Sending changes"
        case "downloading": return "Copying offline files"
        default: return "Connecting to your Mac"
        }
    }
    private func destination(_ context:ActivityViewContext<SyncActivityAttributes>)->URL? {
        if let id=context.state.jobID {return URL(string:"musicos://activity/\(id)")}
        return URL(string:"musicos://activity/sync")
    }
    private func statusIcon(_ phase: String) -> String {
        switch phase {
        case "complete": return "checkmark.circle.fill"
        case "error", "approval": return "exclamationmark.circle"
        case "cancelled", "cancelling": return "pause.circle"
        default: return "arrow.down.circle"
        }
    }
    private func label(_ context: ActivityViewContext<SyncActivityAttributes>) -> String {
        if context.isStale { return "Waiting for an update" }
        return context.state.message.isEmpty ? title(context.state.phase) : context.state.message
    }
    @ViewBuilder private func rail(_ context: ActivityViewContext<SyncActivityAttributes>) -> some View {
        if context.state.total > 0 { ProgressView(value:context.state.fraction).tint(blue) }
    }
    var body:some WidgetConfiguration {
        ActivityConfiguration(for:SyncActivityAttributes.self){context in
            VStack(alignment:.leading,spacing:12){
                HStack{Image(systemName:statusIcon(context.state.phase)).foregroundStyle(blue);Text("musicOS").font(.custom("Figtree",size:17).weight(.semibold));Spacer();Text(context.attributes.computer).font(.custom("Figtree",size:13)).foregroundStyle(.secondary)}
                Text(label(context)).font(.custom("Figtree",size:15)).lineLimit(1)
                rail(context)
                HStack{Text(title(context.state.phase));Spacer();if context.state.total>0{Text("\(context.state.completed) / \(context.state.total)").monospacedDigit()}}
                    .font(.custom("Figtree",size:12)).foregroundStyle(.secondary)
            }.padding(18).activityBackgroundTint(Color(red:0.07,green:0.11,blue:0.15)).activitySystemActionForegroundColor(.white).widgetURL(destination(context))
        } dynamicIsland:{context in
            DynamicIsland{
                DynamicIslandExpandedRegion(.leading){Label("musicOS",systemImage:statusIcon(context.state.phase)).font(.custom("Figtree",size:15).weight(.semibold)).foregroundStyle(blue)}
                DynamicIslandExpandedRegion(.trailing){Text(context.state.total>0 ? "\(Int(context.state.fraction*100))%" : "Sync").font(.custom("Figtree",size:15)).monospacedDigit()}
                DynamicIslandExpandedRegion(.center){Text(context.attributes.computer).font(.custom("Figtree",size:12)).foregroundStyle(.secondary)}
                DynamicIslandExpandedRegion(.bottom){VStack(alignment:.leading,spacing:10){Text(label(context)).font(.custom("Figtree",size:14)).lineLimit(1);rail(context)}.padding(.top,4)}
            }compactLeading:{Image(systemName:statusIcon(context.state.phase)).foregroundStyle(blue)}compactTrailing:{Text(context.state.total>0 ? "\(Int(context.state.fraction*100))%" : context.state.jobID==nil ? "Sync" : "Mac").font(.custom("Figtree",size:11)).monospacedDigit()}minimal:{Image(systemName:statusIcon(context.state.phase)).foregroundStyle(blue)}.widgetURL(destination(context)).keylineTint(blue)
        }
    }
}
