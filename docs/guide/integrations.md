# Optional listening integrations

[Guide home](README.md)

Open **Settings → Integrations**. Both connections are opt-in. Setup details stay collapsed until needed; **Connection details** contains technical status and queued-listen information.

## Discord presence

Create your own Discord application in the [developer portal](https://discord.com/developers/applications), copy its Application ID into MusicOS, and save it. Enable **Discord presence** when ready and leave Discord desktop running.

Presence shares the current song, artist, album, listening time, and matching public album art. MusicOS uses artist and album names to find public artwork in Apple's catalog. It does not upload your local cover files. If Discord is unavailable, playback continues normally.

## Last.fm listening history

Create a [Last.fm API account](https://www.last.fm/api/account/create). Expand **Connect Last.fm**, enter its API key and shared secret, and choose **Authorize MusicOS**. Approve the connection in your browser, return to MusicOS, and finish authorization. The session is stored in the system keychain.

When enabled, listening history sends track metadata after half a song or four minutes. Offline listens wait in a queue. **Disconnect & clear queued listens** removes the connection and its pending listens; local music and playback remain intact.

These are guided setups with user-supplied application credentials, rather than one-click service connections. Never share credentials in bug reports; connection errors belong in Details.

## Listening from an iPhone

1. Connect Last.fm or enable Discord presence in **Settings → Integrations** on the Mac. For Vesktop, enable Rich Presence via arRPC and leave Vesktop open.
2. Start Wi-Fi sharing in **Devices**. Beside your paired iPhone, choose **Allow listening activity**.
3. Open **Settings → Listening sync** on the iPhone and turn on either service. **Check connection** shows whether the Mac is reachable and the services are enabled.

The iPhone keeps eligible Last.fm listens offline and sends them to the paired Mac when it is reachable. Turning off Last.fm on the iPhone clears the listens still waiting there. Discord presence is live while the Mac is reachable and clears on pause or when the connection expires. Playback on the Mac takes priority for Discord. Service credentials stay on the Mac.
