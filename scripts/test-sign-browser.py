#!/usr/bin/env python3
# [INPUT]: sign-browser.py 与临时伪 Mach-O/Framework/Helper 树。
# [OUTPUT]: 验证签名依赖顺序、最小 JIT 范围与符号链接边界，不调用真实 codesign。
# [POS]: 浏览器签名流程的隔离回归检查，与真实发布签名校验互补。
# [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
import importlib.util
from pathlib import Path
import tempfile
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('sign_browser', Path(__file__).with_name('sign-browser.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
with tempfile.TemporaryDirectory() as temporary:
    app = Path(temporary) / 'Browser.app'
    framework = app / 'Contents/Frameworks/Chromium.framework'
    binaries = [app / 'Contents/MacOS/Chromium', framework / 'Versions/A/Chromium']
    helpers = []
    for label in ['Renderer', 'GPU', 'Plugin']:
        helper = framework / f'Versions/A/Helpers/Chromium Helper ({label}).app'
        helpers.append(helper)
        binaries.append(helper / 'Contents/MacOS/Helper')
    for binary in binaries:
        binary.parent.mkdir(parents=True, exist_ok=True)
        binary.write_bytes(b'\xcf\xfa\xed\xfe')
    (framework / 'Versions/Current').symlink_to('A')
    calls = []
    with patch.object(module, 'APP', app), patch.object(module, 'sign', side_effect=lambda path, jit=False: calls.append((path, jit))), patch.object(module.subprocess, 'run') as verify:
        module.main()
    assert calls[-1] == (app, False)
    assert len(calls) == len(binaries) + len(helpers) + 2, calls
    order = [path for path, _ in calls]
    assert all(order.index(binary) < order.index(framework) for binary in binaries)
    assert all(order.index(helper) < order.index(framework) for helper in helpers)
    assert {path for path, jit in calls if jit} == set(helpers[:2])
    verify.assert_called_once_with(['/usr/bin/codesign', '--verify', '--deep', '--strict', str(app)], check=True)
print('Browser signing order, JIT scope and symlink regression passed')
