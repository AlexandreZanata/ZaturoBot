#!/usr/bin/env python3
"""Rebuild the six-page branded ZaturoBot system/design document.

Uses ReportLab and system Noto fonts. Original owner artwork remains unchanged.
"""
from __future__ import annotations
import hashlib
import json
import re
from pathlib import Path
from xml.etree import ElementTree

from reportlab.pdfgen import canvas
from reportlab.lib.colors import HexColor
from reportlab.lib.styles import ParagraphStyle
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import Paragraph

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'output/pdf/zaturobot-system-design.pdf'
OUT.parent.mkdir(parents=True, exist_ok=True)
TOKENS = json.loads((ROOT / 'assets/brand/tokens.json').read_text())
W, H = 1200, 800
M = 56
COL = TOKENS['colors']
GOLD, BG, FG = [HexColor(COL[k]) for k in ('brand', 'background', 'foreground')]
SURFACE, STROKE, MUTED = [HexColor(COL[k]) for k in ('surface', 'stroke', 'muted')]
DIM = HexColor('#8B8B96')
FONT_DIR = Path('/usr/share/fonts/truetype/noto')
FONTS = {
    'Sans': FONT_DIR / 'NotoSans-Regular.ttf',
    'SansBold': FONT_DIR / 'NotoSans-Bold.ttf',
    'Mono': FONT_DIR / 'NotoSansMono-Regular.ttf',
    'MonoBold': FONT_DIR / 'NotoSansMono-Bold.ttf',
}
for name, path in FONTS.items():
    if not path.is_file():
        raise RuntimeError(f'Required embedded fallback font is missing: {path.name}')
    pdfmetrics.registerFont(TTFont(name, str(path)))
pdfmetrics.registerFontFamily('Sans', normal='Sans', bold='SansBold', italic='Sans', boldItalic='SansBold')

SVG = ElementTree.parse(ROOT / 'assets/brand/zaturobot-icon.svg').getroot()
SVG_PATHS = [element.attrib['d'] for element in SVG.iter() if element.tag.endswith('path')]
SVG_SIZE = float(SVG.attrib['viewBox'].split()[2])

CAN = canvas.Canvas(str(OUT), pagesize=(W, H), pageCompression=1)
CAN.setTitle('ZaturoBot - System Design and Design System')
CAN.setAuthor('ZaturoBot Engineering')
CAN.setSubject('Native clients, bounded services, private Android viewing and operational contracts')
CAN.setCreator('Editable ReportLab generator - owner-provided SVG and brand tokens')
TEXT_BOXES = []
PAGE = 0


def rect(x, y, width, height, fill=None, stroke=None, radius=0, line=1):
    CAN.setLineWidth(line)
    if fill is not None:
        CAN.setFillColor(fill)
    if stroke is not None:
        CAN.setStrokeColor(stroke)
    if radius:
        CAN.roundRect(x, H-y-height, width, height, radius, fill=int(fill is not None), stroke=int(stroke is not None))
    else:
        CAN.rect(x, H-y-height, width, height, fill=int(fill is not None), stroke=int(stroke is not None))


def line(x1, y1, x2, y2, color=STROKE, width=1, dash=None):
    CAN.saveState()
    CAN.setLineWidth(width)
    CAN.setStrokeColor(color)
    if dash:
        CAN.setDash(dash)
    CAN.line(x1, H-y1, x2, H-y2)
    CAN.restoreState()


def text(x, y, value, size=12, color=FG, font='Sans', spacing=0):
    value = str(value)
    width = pdfmetrics.stringWidth(value, font, size) + max(0, len(value)-1)*spacing
    TEXT_BOXES.append((PAGE, x, y, width, size*1.24, value[:90]))
    CAN.saveState()
    CAN.setFillColor(color)
    item = CAN.beginText(x, H-y-size)
    item.setFont(font, size)
    item.setCharSpace(spacing)
    item.textOut(value)
    CAN.drawText(item)
    CAN.restoreState()
    return width


def para(x, y, width, value, size=12.3, color=FG, leading=None, font='Sans'):
    style = ParagraphStyle('body', fontName=font, fontSize=size, leading=leading or size*1.46,
                           textColor=color, spaceAfter=0, allowWidows=0, allowOrphans=0)
    item = Paragraph(value, style)
    _, height = item.wrap(width, H*2)
    item.drawOn(CAN, x, H-y-height)
    TEXT_BOXES.append((PAGE, x, y, width, height, re.sub('<[^>]*>', '', value)[:90]))
    return height


def label(x, y, value, color=GOLD, size=9.5):
    return text(x, y, value.upper(), size, color, 'MonoBold', 1.1)


def icon(x, y, size, color=FG):
    CAN.saveState()
    CAN.translate(x, H-y)
    CAN.scale(size/SVG_SIZE, -size/SVG_SIZE)
    CAN.setFillColor(color)
    for data in SVG_PATHS:
        tokens = re.findall(r'[MLZ]|-?\d+(?:\.\d+)?', data)
        path = CAN.beginPath()
        index = 0
        while index < len(tokens):
            command = tokens[index]
            index += 1
            if command == 'Z':
                path.close()
                continue
            if command not in ('M', 'L'):
                raise ValueError('Owner SVG contains an unsupported drawing command')
            while index < len(tokens) and tokens[index] not in ('M', 'L', 'Z'):
                px, py = float(tokens[index]), float(tokens[index+1])
                (path.moveTo if command == 'M' else path.lineTo)(px, py)
                command = 'L'
                index += 2
        CAN.drawPath(path, fill=1, stroke=0, fillMode=0)
    CAN.restoreState()


def arrow(x1, y1, x2, y2, color=GOLD, width=1.6, double=False):
    from math import atan2, cos, sin
    line(x1, y1, x2, y2, color, width)
    def head(x, y, angle):
        length = 7
        for turn in (-0.46, 0.46):
            line(x, y, x-length*cos(angle+turn), y-length*sin(angle+turn), color, width)
    angle = atan2(y2-y1, x2-x1)
    head(x2, y2, angle)
    if double:
        head(x1, y1, angle+3.141592653589793)


def footer(section, sources=None):
    if sources:
        for idx, (caption, url) in enumerate(sources):
            y = 721 + idx*12
            text(M, y, caption + '  ' + url, 7.8, DIM, 'Mono')
            CAN.linkURL(url, (M, H-y-10, W-M, H-y+1), relative=0, thickness=0)
    line(M, 757, W-M, 757, STROKE)
    text(M, 772, 'ZATUROBOT  /  SYSTEM DESIGN + DESIGN SYSTEM  /  13 NOV 2026', 8.3, DIM, 'Mono')
    right = f'{PAGE:02d} / 06'
    text(W-M-pdfmetrics.stringWidth(right, 'Mono', 9), 771, right, 9, MUTED, 'Mono')


def start(section, title=None, deck=None):
    global PAGE
    PAGE += 1
    rect(0, 0, W, H, fill=BG)
    label(M, 34, 'ZaturoBot  /  ' + section)
    if title:
        text(M, 75, title, 37, FG, 'SansBold')
    if deck:
        para(M, 132, W-2*M, deck, 13.5, MUTED, 20)


def end(section, sources=None):
    footer(section, sources)
    CAN.showPage()


def contrast(a, b):
    def luminance(hex_value):
        values = [int(hex_value.lstrip('#')[i:i+2], 16)/255 for i in (0, 2, 4)]
        values = [v/12.92 if v <= .04045 else ((v+.055)/1.055)**2.4 for v in values]
        return sum(v*k for v, k in zip(values, (.2126, .7152, .0722)))
    bright, dark = sorted((luminance(a), luminance(b)), reverse=True)
    return (bright+.05)/(dark+.05)


# 01 - Product identity and foundations.
start('Identity / 0.4 foundation')
text(795, 35, 'SYSTEM DESIGN + DESIGN SYSTEM', 9.3, MUTED, 'Mono')
icon(50, 110, 252)
brand_x, brand_y = 303, 145
brand_width = text(brand_x, brand_y, 'Zaturo', 80, FG, 'SansBold')
text(brand_x+brand_width+3, brand_y, 'Bot', 80, GOLD, 'SansBold')
para(306, 250, 780, 'Your personal Linux assistant.<br/>Native clients. Bounded services. Private control.', 23, FG, 32)
rect(307, 342, 216, 31, GOLD, radius=6)
label(323, 352, 'INITIAL NATIVE RELEASE', BG, 8.4)
text(548, 348, 'Owner-first execution. Evidence-first reporting.', 13, MUTED)
line(M, 405, W-M, 405)

swatches = [
    (56, GOLD, '#E5B900', 'Zaturo gold', 'Primary action / focus / meaningful emphasis'),
    (430, BG, '#0B0B0D', 'Foundation black', 'Background / private panel / visual calm'),
    (804, FG, '#FFFFFF', 'Contrast white', 'Core text / original mark / clear hierarchy'),
]
for x, color, code, title, detail in swatches:
    rect(x, 440, 58, 58, color, STROKE if code != '#E5B900' else None, 9)
    text(x+75, 440, title, 16, FG, 'SansBold')
    text(x+75, 469, code, 12, GOLD if code == '#E5B900' else MUTED, 'Mono')
    para(x, 517, 326, detail, 11.3, MUTED, 16)

label(M, 586, 'Typography and rhythm')
text(M, 610, 'Inter', 23, FG, 'SansBold')
text(222, 612, 'Interface target', 11.5, MUTED)
text(M, 650, 'JetBrains Mono', 17.5, FG, 'MonoBold')
text(306, 654, 'Code / logs target', 11.3, MUTED)
para(56, 690, 528, 'This PDF embeds Noto Sans and Noto Sans Mono as verified local fallbacks. Inter and JetBrains Mono are not bundled or claimed installed.', 10.1, DIM, 14)
label(650, 586, 'Accessible use')
white_black = contrast('#FFFFFF', '#0B0B0D')
gold_black = contrast('#E5B900', '#0B0B0D')
white_gold = contrast('#FFFFFF', '#E5B900')
text(650, 610, f'White / black  {white_black:.2f}:1', 12.2, FG, 'Mono')
text(650, 638, f'Gold / black   {gold_black:.2f}:1', 12.2, GOLD, 'Mono')
para(650, 671, 477, f'Use dark text on gold buttons. White on gold is only {white_gold:.2f}:1 and is unsuitable for small functional text. Space on a 4 / 8 / 12 / 16 / 24 / 32 / 48 scale.', 10.8, MUTED, 16)
end('Identity')

# 02 - Owner interface and history model.
start('Native experience', 'One hub. The day in focus.',
      'Chat and an activity feed share a durable history. Today is the initial view; date, type and search filters retrieve earlier work.')
rect(56, 195, 792, 411, SURFACE, STROKE, 14)
rect(56, 195, 792, 43, None, STROKE, 14)
icon(71, 203, 31)
text(111, 207, 'ZaturoBot', 17, FG, 'SansBold')
text(593, 212, 'Conceptual layout - not live data', 8.8, DIM, 'Mono')
line(224, 238, 224, 606)
rect(68, 255, 144, 34, GOLD, radius=7)
text(84, 264, 'Hub', 12, BG, 'SansBold')
for index, name in enumerate(('Services', 'Workers', 'Files', 'Settings')):
    text(84, 310+index*41, name, 11.5, MUTED)
label(246, 258, 'Today')
text(246, 283, '13 November 2026', 22, FG, 'SansBold')
for x, width, title in [(246, 139, '2026-11-13'), (397, 108, 'All activity'), (517, 303, 'Search history...')]:
    rect(x, 326, width, 31, BG, STROKE, 6)
    text(x+10, 335, title, 9.6, MUTED, 'Mono' if x == 246 else 'Sans')
rect(463, 378, 357, 43, HexColor('#24242A'), radius=9)
text(479, 392, 'Check the work PC.', 11.4)
line(255, 450, 255, 541, STROKE, 1.1)
CAN.setFillColor(GOLD)
CAN.circle(255, H-459, 4.3, fill=1, stroke=0)
text(275, 445, 'Diagnostic finished', 13, FG, 'SansBold')
text(275, 470, 'Typed execution evidence is saved with this task.', 10.6, MUTED)
CAN.setFillColor(GOLD)
CAN.circle(255, H-526, 4.3, fill=1, stroke=0)
rect(276, 508, 56, 47, BG, STROKE, 6)
line(285, 545, 299, 528, MUTED, 1)
line(299, 528, 321, 545, MUTED, 1)
text(348, 512, 'Optional image evidence', 12.2, FG, 'SansBold')
text(348, 537, 'Normal artifacts only; private screens use another path.', 9.4, MUTED)
rect(245, 567, 575, 27, BG, STROKE, 6)
text(258, 575, 'Describe the next task...', 9.6, DIM)
rect(785, 570, 30, 21, GOLD, radius=4)
arrow(794, 580, 806, 580, BG, 1.4)

notes = [
    ('Today by default', 'Persist UTC; present the requested local day using the client offset. Initial owner zone: America/Cuiaba.'),
    ('History remains available', 'Changing the day is a query, not deletion. Completed tasks and ordinary evidence stay linked.'),
    ('Completion follows evidence', 'Admitted video work and finished video work are different events. The feed reports actual job transitions.'),
    ('Private viewing is separate', 'Sensitive phone images never become feed thumbnails, model context or ordinary stored artifacts.'),
]
y = 198
for title, body in notes:
    label(888, y, title, GOLD, 8.2)
    height = para(888, y+24, 249, body, 10.8, MUTED, 16)
    y += height+53
text(56, 646, 'A saved chat message is not an executed task.', 24, FG, 'SansBold')
para(56, 688, 1082, 'Only explicitly supported typed actions are dispatched. Other messages remain in history with an honest, unexecuted status. The owner interface defaults to Brazilian Portuguese; labels here are in English for design review.', 11.6, MUTED, 18)
end('Native experience')

# 03 - Runtime architecture and trust boundaries.
start('Runtime architecture', 'Keep coordination small. Execute at the edge.',
      'Linux uses Rust + Slint. Android uses Kotlin. Both reach a private hub; notebook services own computation, credentials and USB hardware.')
boxes = [(56, 220, 250, 285), (393, 220, 310, 285), (790, 220, 354, 285)]
for x, y, w, h in boxes:
    rect(x, y, w, h, SURFACE, STROKE, 12)
label(76, 240, 'Owner clients')
text(76, 275, 'Linux desktop', 20, FG, 'SansBold')
text(76, 309, 'Rust + Slint', 12.3, GOLD, 'Mono')
text(76, 359, 'Android app', 20, FG, 'SansBold')
text(76, 393, 'Kotlin / Android only', 11.2, GOLD, 'Mono')
para(76, 439, 211, 'Scoped app credentials<br/>No provider key in a UI bundle', 10.7, MUTED, 17)
label(415, 240, 'Oracle VPS / 1 GB')
text(415, 275, 'Rust hub + SQLite', 23, FG, 'SansBold')
for index, item in enumerate(('Tasks, feed and worker metadata', 'Authentication, leases and audit', 'Disk: ordinary encrypted file spool', 'RAM: expiring private ciphertext')):
    para(415, 329+index*38, 266, item, 11.3, MUTED, 16)
label(812, 240, 'Linux notebook / 22 GB')
text(812, 275, 'Rust worker', 23, FG, 'SansBold')
for index, item in enumerate(('Outbound task connection', 'Local journal and safe adapters', 'Keys and account profiles stay local', 'age encryption before file upload')):
    para(812, 329+index*38, 310, item, 11.4, MUTED, 16)
arrow(311, 346, 387, 346, GOLD, 1.7)
text(317, 310, 'SSH 22', 10, GOLD, 'MonoBold')
text(318, 362, 'outbound', 8.6, MUTED, 'Mono')
arrow(784, 346, 709, 346, GOLD, 1.7)
text(716, 310, 'SSH 22', 10, GOLD, 'MonoBold')
text(715, 362, 'outbound', 8.6, MUTED, 'Mono')
line(925, 505, 925, 528, STROKE)
line(925, 528, 898, 528, STROKE)
arrow(898, 528, 898, 553, GOLD)
line(1089, 505, 1089, 528, STROKE)
arrow(1089, 528, 1089, 553, GOLD)
rect(790, 557, 224, 133, None, STROKE, 10)
label(806, 574, 'Independent heavy job', size=7.7)
text(806, 600, 'YoutubeBot actor', 17.5, FG, 'SansBold')
para(806, 633, 187, 'Separate systemd user cgroup.<br/>Video / browser / upload workload.', 10, MUTED, 15)
rect(1042, 557, 102, 133, None, STROKE, 12)
rect(1055, 577, 76, 78, BG, STROKE, 7)
line(1082, 566, 1103, 566, MUTED, 1.2)
text(1069, 669, 'USB phone', 9.3, FG, 'SansBold')
label(56, 554, 'A deliberate resource boundary')
text(56, 584, 'The VPS does not become a remote workstation.', 22, FG, 'SansBold')
para(56, 624, 640, 'No video rendering, browser, ADB server or account credentials run there. Heavy video jobs are independent of the lightweight worker\'s 512 MiB memory ceiling. Private SSH host checks stay enabled; home-router inbound ports stay closed.', 11.6, MUTED, 18)
text(56, 703, 'PROTOCOL 2  /  CAPABILITY-GATED SERVICES  /  HUB SCHEMA 5  /  WORKER SCHEMA 6', 9.6, GOLD, 'Mono')
end('Architecture', [('Slint Rust UI', 'https://docs.slint.dev/latest/docs/slint/guide/getting-started/')])

# 04 - YouTube preservation, accounts and asynchronous work.
start('YoutubeBot service', 'Preserve the pipeline. Separate the identity.',
      'The account manager and service boundary surround the existing extension, video generation and uploader. Original local source files remain preserved.')
steps = [
    ('01', 'Choose an account', 'Use the real Google browser consent window; no embedded password form.'),
    ('02', 'Bind the channel', 'PKCE + state protect authorization. Verify the actual YouTube channel ID.'),
    ('03', 'Keep profiles apart', 'Separate Google identity, channel bindings and browser profile directories.'),
    ('04', 'Manage the library', 'Organize titles, topics, runs, schedule slots and bounded artifact summaries.'),
]
for index, (number, title, body) in enumerate(steps):
    x = 56+index*280
    label(x, 201, number)
    text(x, 232, title, 17.2, FG, 'SansBold')
    para(x, 270, 245, body, 11.4, MUTED, 17)
    if index<3:
        arrow(x+250, 239, x+270, 239, STROKE, 1.2)
line(56, 351, 1144, 351)
label(56, 376, 'Existing production path / behavior retained')
pipeline = [('Browser extension', 56, 231), ('Script + audio', 342, 210), ('Video rendering', 607, 217), ('Upload + schedule', 879, 265)]
for title, x, width in pipeline:
    rect(x, 409, width, 47, SURFACE, STROKE, 8)
    text(x+16, 422, title, 13.3, FG, 'SansBold')
for left, right in zip(pipeline, pipeline[1:]):
    arrow(left[1]+left[2]+7, 433, right[1]-8, 433, GOLD, 1.3)
label(56, 494, 'Durable background lifecycle')
for x, title, detail in [(56, 'accepted', 'Durable admission'), (286, 'running', 'Actual actor started'), (546, 'completed', 'Scheduled-item evidence')]:
    text(x, 525, title, 18, GOLD if title=='completed' else FG, 'MonoBold')
    text(x, 560, detail, 10.6, MUTED)
arrow(204, 541, 264, 541)
arrow(432, 541, 520, 541)
line(355, 585, 355, 603, STROKE)
line(355, 603, 605, 603, STROKE)
arrow(355, 603, 355, 621, STROKE, 1.2)
arrow(605, 603, 605, 621, STROKE, 1.2)
text(286, 631, 'failed', 17, FG, 'MonoBold')
text(546, 631, 'uncertain', 17, FG, 'MonoBold')
text(286, 661, 'Known failure', 10.6, MUTED)
text(546, 661, 'Status-only reconciliation', 10.2, MUTED)
rect(829, 495, 315, 204, SURFACE, STROKE, 12)
label(847, 513, 'Independent actor limits', size=8)
text(847, 545, '12G / 16G', 31, FG, 'SansBold')
text(847, 590, 'MemoryHigh / MemoryMax', 10.5, MUTED, 'Mono')
text(847, 620, '200% CPU / 12 h / no restart', 10.3, GOLD, 'Mono')
para(847, 651, 275, 'Never report admission as a finished video. A lost response queries the same operation; it never blindly submits again.', 10.2, MUTED, 14.5)
end('YouTube', [('Google desktop OAuth', 'https://developers.google.com/identity/protocols/oauth2/native-app')])

# 05 - The private Android image route.
start('Sensitive-screen boundary', 'Private screens take a different route.',
      'A screenshot can contain authentication material. The user authorizes a finite view; pixels never enter ordinary history, a provider prompt or the normal file spool.')
flow = [
    ('Owner grant', 'Exact phone and viewer.<br/>Explicit confirmation.<br/>Immediate or 5-120 s watch.'),
    ('USB capture', 'Authorized, unlocked phone.<br/>PNG in protected memory.<br/>Serial + revocable lease.'),
    ('age encryption', 'Registered viewer recipient.<br/>Private key at endpoint.<br/>No OCR. No LLM.'),
    ('RAM relay', 'At most two sessions.<br/>Cipher expires in <=60 s.<br/>No SQL or image files.'),
    ('Private viewer', 'Linux desktop decrypts.<br/>Consume once; clear on<br/>expiry, close or focus loss.'),
]
for index, (title, body) in enumerate(flow):
    x=56+index*225
    label(x, 211, f'{index+1:02d}')
    text(x, 245, title, 17, FG, 'SansBold')
    para(x, 289, 185, body, 11.2, MUTED, 18)
    if index<4:
        arrow(x+190, 260, x+215, 260, GOLD, 1.3)
line(56, 390, 1144, 390)
text(56, 425, 'History stores metadata only.', 26, FG, 'SansBold')
para(56, 471, 523, 'The acceptance contract rejects extra image fields, code strings and arbitrary private captions, even when they fit inside a small JSON payload.', 11.5, MUTED, 18)
rect(56, 537, 531, 135, SURFACE, STROKE, 9)
code = ['{', '  "session_id": "00000000-0000-4000-8000-000000000000",', '  "delivery": "ephemeral_encrypted_memory",', '  "max_view_seconds": 60', '}']
for index, row in enumerate(code):
    text(72, 551+index*21, row, 9.2, GOLD if index in (2,3) else MUTED, 'Mono')
label(646, 431, 'An image change is not an SMS detector')
para(646, 462, 492, 'A clock, animation or app switch can trigger the finite watch. It returns at most the first changed screen; an unchanged interval ends normally. No notification harvesting or authentication-code extraction is implemented.', 12, MUTED, 18.5)
label(646, 553, 'Respect Android protections')
para(646, 582, 492, 'The owner prepares the SMS screen and performs any required unlock or USB RSA consent. No root, default-SMS-handler change, lock-screen bypass or FLAG_SECURE workaround is introduced.', 11.7, MUTED, 18)
text(56, 694, 'ANDROID APP: CHAT + FEED     /     PRIVATE AGE IMAGE VIEWER: LINUX DESKTOP', 9.6, GOLD, 'Mono')
end('Private screens', [
    ('ADB screen capture', 'https://developer.android.com/tools/adb'),
    ('Secure windows', 'https://developer.android.com/reference/android/view/WindowManager.LayoutParams#FLAG_SECURE'),
])

# 06 - Contracts, limits and release evidence.
start('Operational contracts', 'Deploy additively. Verify the actual outcome.',
      'The work PC is already enrolled on 0.3. The 0.4 update preserves its private keys, identity and journal; source changes and local assets follow separate release paths.')
label(56, 202, 'API surface')
api = [
    ('/v1/chat', 'Durable request + explicit typed actions'),
    ('/v1/feed', 'Day, type, search and bounded history pages'),
    ('/v1/services', 'Worker capability inventory'),
    ('/v1/device/activity', 'Actual background service transitions'),
    ('/v1/private-views', 'Owner grant + one-time, encrypted RAM delivery'),
    ('/v1/device/transfers', 'Resumable ordinary age-encrypted files'),
]
for index,(route,detail) in enumerate(api):
    y=235+index*47
    text(56,y,route,11.7,FG,'MonoBold')
    text(56,y+22,detail,10.8,MUTED)
label(650, 202, 'File and image limits')
text(650, 234, '2 GiB file / 8 GiB spool / 24 h', 16.5, FG, 'SansBold')
para(650, 270, 487, 'Ordinary transfer defaults; 4 MiB resumable chunks. Quota is released after expired ciphertext is actually removed.', 11.5, MUTED, 17.5)
text(650, 329, '8 MiB PNG / 2 private sessions', 16.5, FG, 'SansBold')
para(650, 365, 487, 'Cipher cap: 8 MiB + 64 KiB. One-time grant and receipt, active expiry and no-store responses; no sensitive screenshot disk cache.', 11.5, MUTED, 17.5)
label(650, 427, 'Local asset migration / separate profile')
para(650, 456, 487, '12,455 captured local files: 8.90 GB source, 7.71 GB outer age cipher. Eight parts of at most 1 GiB are encrypted again by the file transport. Temporary VPS policy: 16 GiB spool and seven-day retention.', 11.3, MUTED, 17.4)
line(56, 543, 1144, 543)
label(56, 566, 'Implemented and tested in code')
para(56, 598, 512, 'Rust task/feed/service contracts; Linux native client; Kotlin Android debug build; bounded adapters; replay fencing; synthetic-device privacy tests. Native binaries and APK are build artifacts, not proof of real account or hardware consent.', 11.6, MUTED, 18)
label(650, 566, 'Physical acceptance still required')
para(650, 598, 487, 'Google browser consent, the exact USB phone, protected-screen behavior and real video generation/upload have not been exercised. Verify them on the work PC before claiming production success. Rebuildable caches, models and virtual environments are excluded from migration.', 11.6, MUTED, 18)
text(56, 705, 'NO CREDENTIALS IN GIT  /  NO PUBLIC ADB  /  NO BLIND SIDE-EFFECT REPLAY', 10, GOLD, 'MonoBold')
end('Operational contracts')

# Mechanical page-bound checks supplement, but never replace, raster inspection.
violations=[]
for page,x,y,width,height,sample in TEXT_BOXES:
    if x < 35 or x+width > W-35 or y < 20 or y+height > H-12:
        violations.append((page,round(x,1),round(y,1),round(width,1),round(height,1),sample))
if violations:
    raise RuntimeError('Text exceeds the page safety bounds: '+repr(violations))
CAN.save()
print(json.dumps({'pdf':str(OUT),'pages':PAGE,'bytes':OUT.stat().st_size,'sha256':hashlib.sha256(OUT.read_bytes()).hexdigest(),'embedded_fallback_fonts':['Noto Sans','Noto Sans Mono'],'contrast':{'white_on_black':round(white_black,2),'gold_on_black':round(gold_black,2),'white_on_gold':round(white_gold,2)}},indent=2))
