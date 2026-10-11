// [INPUT]: 用户品牌图源、输出 PNG 路径与目标像素尺寸。
// [OUTPUT]: macOS 图标网格中的圆角方形 RGBA PNG，保留原图与柔和投影。
// [POS]: A/B 图标共用生成器；只读取 assets/ 品牌图，由 make-icons.sh 生成各尺寸与 ICNS。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import AppKit
let arguments = CommandLine.arguments
guard arguments.count == 4,
      let pixels = Int(arguments[3]), pixels > 0,
      let image = NSImage(contentsOfFile: arguments[1]) else {
    fputs("用法: make-icon.swift 源图.png 输出.png 尺寸\n", stderr)
    exit(2)
}
let size = CGFloat(pixels), scale = size / 1024
let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: pixels, pixelsHigh: pixels,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
bitmap.size = NSSize(width: size, height: size)
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
NSGraphicsContext.current?.imageInterpolation = .high
let rect = NSRect(x: 100 * scale, y: 100 * scale, width: 824 * scale, height: 824 * scale)
let rounded = NSBezierPath(roundedRect: rect, xRadius: 185 * scale, yRadius: 185 * scale)
// ----- 仅投影和平台圆角，品牌图内容不重绘 -----
NSGraphicsContext.saveGraphicsState()
let shadow = NSShadow()
shadow.shadowColor = NSColor(white: 0, alpha: 0.25)
shadow.shadowOffset = NSSize(width: 0, height: -10 * scale)
shadow.shadowBlurRadius = 24 * scale
shadow.set()
NSColor.white.setFill()
rounded.fill()
NSGraphicsContext.restoreGraphicsState()
NSGraphicsContext.saveGraphicsState()
rounded.addClip()
image.draw(in: rect, from: .zero, operation: .sourceOver, fraction: 1)
NSGraphicsContext.restoreGraphicsState()
NSGraphicsContext.restoreGraphicsState()
guard let data = bitmap.representation(using: .png, properties: [:]) else { exit(1) }
try data.write(to: URL(fileURLWithPath: arguments[2]))
