# Rodio 0.19 desktop seek fix

Vendored from crates.io rodio 0.19.0, retaining its MIT/Apache licenses.
The sole behavior change is gated by `desktop-seek-length`: the Symphonia
Read+Seek wrapper records the stream length and restores the original position.
Symphonia FLAC binary-search seeking requires this length. Tauri enables this
feature and symphonia-all; the separately built embedded runtime enables neither
and retains its original decoder behavior. Revisit when upgrading Rodio.
