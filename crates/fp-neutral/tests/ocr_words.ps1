# Texterkennung (Windows.Media.Ocr) für die Neutralitätsprüfung der Bilder (tests\neutral.rs).
# Eingabe: Textdatei mit einem PNG-Pfad je Zeile. Ausgabe (UTF-8): je erkanntem Wort eine Zeile
#   <Pfad>`t<Zeilentext>`t<Wort>`t<x>`t<y>`t<Breite>`t<Höhe>   (Bildpixel, unskaliert)
# Die Suche nach gesperrten Namen macht der Aufrufer mit fp_neutral::find_in_bytes – dieses Skript
# kennt keine Namen. Nicht lesbare Bilder: x = y = -1, Fehlertext statt Zeilentext. Exit-Code 3: keine Texterkennung auf diesem System.
param([Parameter(Mandatory)][string]$List, [Parameter(Mandatory)][string]$Out, [int]$Scale = 2)
$ErrorActionPreference = 'Stop'
try {
  Add-Type -AssemblyName System.Runtime.WindowsRuntime
  $null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
  $null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation, ContentType = WindowsRuntime]
  $null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
  $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
  if ($null -eq $engine) { exit 3 }
} catch { exit 3 }
$asTask = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($op, [Type]$t) { $task = $asTask.MakeGenericMethod($t).Invoke($null, @($op)); $task.Wait() | Out-Null; $task.Result }
$max = [Windows.Media.Ocr.OcrEngine]::MaxImageDimension
$sw = [System.IO.StreamWriter]::new($Out, $false, [System.Text.UTF8Encoding]::new($false))
foreach ($p in [System.IO.File]::ReadAllLines($List)) {
  if (-not $p) { continue }
  try {
    $file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($p)) ([Windows.Storage.StorageFile])
    $stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
    $dec = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
    $s = [Math]::Min([double]$Scale, [Math]::Min($max / $dec.PixelWidth, $max / $dec.PixelHeight))
    $tf = [Windows.Graphics.Imaging.BitmapTransform]::new()
    $tf.ScaledWidth = [uint32]($dec.PixelWidth * $s); $tf.ScaledHeight = [uint32]($dec.PixelHeight * $s); $tf.InterpolationMode = 'Fant'
    $bmp = Await ($dec.GetSoftwareBitmapAsync([Windows.Graphics.Imaging.BitmapPixelFormat]::Bgra8, [Windows.Graphics.Imaging.BitmapAlphaMode]::Premultiplied, $tf, [Windows.Graphics.Imaging.ExifOrientationMode]::IgnoreExifOrientation, [Windows.Graphics.Imaging.ColorManagementMode]::DoNotColorManage)) ([Windows.Graphics.Imaging.SoftwareBitmap])
    $res = Await ($engine.RecognizeAsync($bmp)) ([Windows.Media.Ocr.OcrResult])
    foreach ($line in $res.Lines) {
      foreach ($w in $line.Words) {
        $r = $w.BoundingRect
        $sw.WriteLine("$p`t$($line.Text)`t$($w.Text)`t$([int]($r.X / $s))`t$([int]($r.Y / $s))`t$([int][Math]::Ceiling($r.Width / $s))`t$([int][Math]::Ceiling($r.Height / $s))")
      }
    }
    $stream.Dispose()
  } catch { $sw.WriteLine("$p`t$($_.Exception.Message)`t`t-1`t-1`t0`t0") }
}
$sw.Close()
