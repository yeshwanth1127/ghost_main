# Clear Windows icon cache so taskbar shows updated app icon
# Run this if the taskbar still shows the old (purple) icon after rebuilding

Write-Host "Clearing Windows icon cache..."
ie4uinit -show
Write-Host "Done. You may need to:"
Write-Host "  1. Unpin Ghost from taskbar"
Write-Host "  2. Close Ghost completely"
Write-Host "  3. Run: npm run tauri dev"
Write-Host "  4. Pin Ghost to taskbar again"
