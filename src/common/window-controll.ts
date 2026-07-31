import { 
    getCurrentWindow, 
    LogicalPosition, 
    availableMonitors, 
    cursorPosition    
} from '@tauri-apps/api/window';

export async function popupPanelNearCursor() {
    const win = getCurrentWindow();

    // 1. Fetch physical mouse coordinates
    const mouse = await cursorPosition(); 
    
    // 2. Fetch all active monitors connected to the system
    const monitors = await availableMonitors();
    
    // 3. Find the monitor that physically contains the mouse cursor
    let targetMonitor = monitors[0]; 
    for (const monitor of monitors) {
        const { x, y } = monitor.position;
        const { width, height } = monitor.size;
        
        if (
            mouse.x >= x && 
            mouse.x <= x + width && 
            mouse.y >= y && 
            mouse.y <= y + height
        ) {
            targetMonitor = monitor;
            break;
        }
    }

    if (!targetMonitor) return;

    // 4. Transform physical bounds into logical workspace units
    const scaleFactor = targetMonitor.scaleFactor || 1;
    
    const screenX = targetMonitor.position.x / scaleFactor;
    const screenY = targetMonitor.position.y / scaleFactor;
    const swidth = targetMonitor.size.width / scaleFactor;
    const sheight = targetMonitor.size.height / scaleFactor;

    const mouseLogicalX = mouse.x / scaleFactor;
    const mouseLogicalY = mouse.y / scaleFactor;

    // 5. Get window logical dimensions
    const winPhysicalSize = await win.innerSize();
    const winWidth = winPhysicalSize.width / scaleFactor;
    const winHeight = winPhysicalSize.height / scaleFactor;

    // 6. FRIENDLY PLACEMENT MECHANISM: 
    // Determine target location dynamically based on cursor quad sector zones
    let x = 0;
    let y = 0;
    const cursorPadding = 12; // Visual spacing between mouse tip and window border

    // --- Dynamic Horizontal Placement ---
    const monitorCenterX = screenX + swidth / 2;
    if (mouseLogicalX > monitorCenterX) {
        // Cursor is on the RIGHT half of the monitor -> Spawn panel safely to the LEFT
        x = mouseLogicalX - winWidth - cursorPadding;
    } else {
        // Cursor is on the LEFT half of the monitor -> Spawn panel safely to the RIGHT
        x = mouseLogicalX + cursorPadding;
    }

    // --- Dynamic Vertical Placement ---
    const monitorCenterY = screenY + sheight / 2;
    if (mouseLogicalY > monitorCenterY) {
        // Cursor is on the BOTTOM half of the monitor -> Spawn panel safely ABOVE
        y = mouseLogicalY - winHeight - cursorPadding;
    } else {
        // Cursor is on the TOP half of the monitor -> Spawn panel safely BELOW
        y = mouseLogicalY + cursorPadding;
    }

    // ===================== Hard Safety Boundary Clamp Fallbacks =====================
    const outerMargin = 8; // Screen border safety padding
    
    // Clamp Horizontal Edges
    if (x + winWidth > screenX + swidth - outerMargin) {
        x = screenX + swidth - winWidth - outerMargin;
    }
    if (x < screenX + outerMargin) {
        x = screenX + outerMargin;
    }
    
    // Clamp Vertical Edges
    if (y + winHeight > screenY + sheight - outerMargin) {
        y = screenY + sheight - winHeight - outerMargin;
    }
    if (y < screenY + outerMargin) {
        y = screenY + outerMargin;
    }

    // 7. Update Window Position and Visibility Frameworks
    await win.setPosition(new LogicalPosition(x, y));
    await win.show();
    await win.setFocus();
}
