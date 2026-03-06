param(
  [int]$Port = 4173
)

Write-Host "AuroraOS desktop preview on http://127.0.0.1:$Port"
Write-Host "Press Ctrl+C to stop."

python -m http.server $Port --directory desktop
