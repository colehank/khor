"""Explicit, frontend-only A/B isolation over the unchanged compiled App."""
import json


CONTENT = 'html :is([data-sticky-fading],[data-sticky-surface]){-webkit-mask-image:none!important;mask-image:none!important}'
HEADER = 'html :is([data-band],[data-sticky-head])::before{-webkit-backdrop-filter:none!important;backdrop-filter:none!important}'
MODES = {
    'normal': ('原效果', ''),
    'content-mask-off': ('关闭正文遮罩', CONTENT),
    'header-blur-off': ('关闭标题模糊', HEADER),
    'sticky-effects-off': ('关闭两层效果', CONTENT + HEADER),
}


def isolate(html, mode, *, frontend, tree):
    if not mode:
        return html
    if not frontend or mode not in MODES:
        raise ValueError('Effect isolation requires frontend Lab and a known effects mode')
    label, css = MODES[mode]
    title = json.dumps(f'Lab {tree[:7]} · {label}', ensure_ascii=False)
    extra = f'<style data-lab-effects="{mode}">{css}</style><script>document.title={title};</script>'
    return html.replace('</head>', extra + '</head>')
