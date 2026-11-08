import ActivityKit
import WidgetKit
import SwiftUI
import MusicOSActivityShared

@main struct MusicOSSyncWidgets:WidgetBundle {var body:some Widget{MusicOSSyncWidget()}}
struct MusicOSSyncWidget:Widget {
    let blue=Color(red:0.58,green:0.82,blue:0.95)
    var body:some WidgetConfiguration {
        ActivityConfiguration(for:SyncActivityAttributes.self){context in
            VStack(alignment:.leading,spacing:12){
                HStack{Image(systemName:"arrow.down.circle.fill").foregroundStyle(blue);Text("musicOS").font(.custom("Figtree",size:17).weight(.semibold));Spacer();Text(context.attributes.computer).font(.custom("Figtree",size:13)).foregroundStyle(.secondary)}
                Text(context.isStale ? "Waiting for the computer…":context.state.message).font(.custom("Figtree",size:15)).lineLimit(1)
                ProgressView(value:context.state.fraction).tint(blue)
                HStack{Text(context.state.phase=="complete" ? "Ready offline":"Copying music");Spacer();if context.state.total>0{Text("\(context.state.completed) / \(context.state.total)").monospacedDigit()}}
                    .font(.custom("Figtree",size:12)).foregroundStyle(.secondary)
            }.padding(18).activityBackgroundTint(Color(red:0.07,green:0.11,blue:0.15)).activitySystemActionForegroundColor(.white).widgetURL(URL(string:"musicos://sync"))
        } dynamicIsland:{context in
            DynamicIsland{
                DynamicIslandExpandedRegion(.leading){Label("musicOS",systemImage:"arrow.down.circle.fill").font(.custom("Figtree",size:15).weight(.semibold)).foregroundStyle(blue)}
                DynamicIslandExpandedRegion(.trailing){Text("\(Int(context.state.fraction*100))%").font(.custom("Figtree",size:15)).monospacedDigit()}
                DynamicIslandExpandedRegion(.center){Text(context.attributes.computer).font(.custom("Figtree",size:12)).foregroundStyle(.secondary)}
                DynamicIslandExpandedRegion(.bottom){VStack(alignment:.leading,spacing:10){Text(context.isStale ? "Waiting for update…":context.state.message).font(.custom("Figtree",size:14)).lineLimit(1);ProgressView(value:context.state.fraction).tint(blue)}.padding(.top,4)}
            }compactLeading:{Image(systemName:"arrow.down").foregroundStyle(blue)}compactTrailing:{Text("\(Int(context.state.fraction*100))%").font(.custom("Figtree",size:11)).monospacedDigit()}minimal:{Image(systemName:"arrow.down.circle.fill").foregroundStyle(blue)}.widgetURL(URL(string:"musicos://sync")).keylineTint(blue)
        }
    }
}
