"""Build the five TMC reader manuals from matching, localized chapter sources."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from xml.sax.saxutils import escape

from reportlab import rl_config
from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (
    Flowable, KeepTogether, PageBreak, Paragraph, SimpleDocTemplate, Spacer, Table, TableStyle,
)
from pypdf import PdfReader, PdfWriter

ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).parent / "source"
OUTPUTS = {
    "es": "Manual_TMC_Comandante.pdf",
    "en": "TMC_Commander_Manual_EN.pdf",
    "fr": "Manuel_TMC_Commandant_FR.pdf",
    "de": "TMC_Kommandantenhandbuch_DE.pdf",
    "pt": "Manual_TMC_Comandante_PT.pdf",
}
SCREENS = [
    ["nav-settings"], ["nav-layouts", "nav-modules"], ["nav-settings"],
    ["nav-dashboard", "nav-ship"], ["nav-ship"], ["nav-navigation"],
    ["nav-exploration"], ["nav-exobiology"], ["nav-engineering"],
    ["nav-combat"], ["nav-missions"], ["nav-mining"], ["nav-trade"],
    ["nav-crime"], ["layout-carrier"], ["layout-colonisation", "layout-powerplay", "layout-bgs"],
    ["nav-settings", "panel-accounts"], ["nav-settings"], ["nav-settings"], ["nav-settings"],
]
NAVY, ORANGE, INK, MUTED = map(colors.HexColor, ["#101a2b", "#d85a12", "#243247", "#526477"])
LIGHT = colors.HexColor("#edf2f7")


def clean(text: str) -> str:
    return text.replace("\u2011", "-").replace("\u2013", "-").replace("\u2014", "-")


def para(text: str, style: ParagraphStyle) -> Paragraph:
    return Paragraph(escape(clean(text)), style)


def register_fonts() -> None:
    folder = Path("C:/Windows/Fonts")
    fallback = Path(rl_config.__file__).parent / "fonts"
    for name, windows, vera in [
        ("TMC", "segoeui.ttf", "Vera.ttf"),
        ("TMC-Bold", "segoeuib.ttf", "VeraBd.ttf"),
        ("TMC-Italic", "segoeuii.ttf", "VeraIt.ttf"),
    ]:
        source = folder / windows
        if not source.exists():
            source = fallback / vera
        pdfmetrics.registerFont(TTFont(name, str(source)))
    pdfmetrics.registerFontFamily("TMC", normal="TMC", bold="TMC-Bold", italic="TMC-Italic")


def labels(language: str) -> dict[str, str]:
    result = {}
    for file in (ROOT / "locales" / language).glob("*.ftl"):
        for line in file.read_text(encoding="utf-8-sig").splitlines():
            match = re.match(r"^([a-z0-9-]+)\s*=\s*(.*)$", line)
            if match:
                result[match[1]] = match[2]
    return result


class Workflow(Flowable):
    def __init__(self, names: list[str], width: float):
        super().__init__()
        self.names, self.width, self.height = names, width, 62

    def draw(self):
        gap, box = 18, (self.width - 36) / 3
        for i, label in enumerate(self.names):
            x = i * (box + gap)
            self.canv.setFillColor(LIGHT)
            self.canv.roundRect(x, 11, box, 44, 5, fill=1, stroke=0)
            self.canv.setFillColor(INK)
            self.canv.setFont("TMC-Bold", 9)
            self.canv.drawCentredString(x + box / 2, 29, label)
            if i < 2:
                self.canv.setStrokeColor(ORANGE)
                self.canv.setLineWidth(1.5)
                self.canv.line(x + box + 4, 33, x + box + 14, 33)
                self.canv.line(x + box + 11, 36, x + box + 14, 33)
                self.canv.line(x + box + 11, 30, x + box + 14, 33)


def build(language: str, data: dict, output: Path, version: str) -> dict:
    width, height = A4
    inner = width - 88
    body = ParagraphStyle("Body", fontName="TMC", fontSize=10.2, leading=14.5,
                          textColor=INK, spaceAfter=9, splitLongWords=True)
    small = ParagraphStyle("Small", parent=body, fontSize=8.4, leading=11.5,
                           textColor=MUTED, spaceAfter=6)
    heading = ParagraphStyle("Heading", parent=body, fontName="TMC-Bold", fontSize=22,
                             leading=26, textColor=NAVY, spaceAfter=14)
    sub = ParagraphStyle("Sub", parent=body, fontName="TMC-Bold", fontSize=11,
                         leading=15, textColor=ORANGE, spaceBefore=12, spaceAfter=7)
    step_style = ParagraphStyle("Step", parent=body, spaceAfter=0)
    number_style = ParagraphStyle("Number", parent=step_style, fontName="TMC-Bold", textColor=ORANGE)
    cover_title = ParagraphStyle("Cover", parent=heading, fontSize=31, leading=36, spaceAfter=23)
    chapters = data["sections"]
    assert len(chapters) == 20
    page_total = len(chapters) + 2

    def page_chrome(canvas, doc):
        canvas.saveState()
        if doc.page > 1:
            canvas.setStrokeColor(ORANGE)
            canvas.setLineWidth(2)
            canvas.line(44, height - 35, width - 44, height - 35)
            canvas.setFillColor(NAVY)
            canvas.setFont("TMC-Bold", 8)
            canvas.drawString(44, height - 27, f"TMC  /  {data['language']}")
            canvas.setFillColor(MUTED)
            canvas.setFont("TMC", 8)
            canvas.drawRightString(width - 44, height - 27, f"v{version}")
        canvas.setStrokeColor(colors.HexColor("#d7e0ea"))
        canvas.setLineWidth(0.5)
        canvas.line(44, 37, width - 44, 37)
        canvas.setFillColor(MUTED)
        canvas.setFont("TMC", 7.6)
        canvas.drawString(44, 24, clean(data["footer"]))
        canvas.drawRightString(width - 44, 24, f"{doc.page:02} / {page_total:02}")
        if doc.page > 2:
            canvas.linkAbsolute("contents", "contents", (44, 15, 220, 34), thickness=0)
        canvas.restoreState()

    doc = SimpleDocTemplate(str(output), pagesize=A4, leftMargin=44, rightMargin=44,
                            topMargin=62, bottomMargin=51, title=f"TMC v{version} - {data['title']}",
                            author="The Multipurpose Companion", subject=data["subtitle"])
    story = [Spacer(1, 50), para("TMC", ParagraphStyle("Brand", parent=heading, fontSize=56, leading=62)),
             para("THE MULTIPURPOSE COMPANION", small), Spacer(1, 22),
             para(data["title"], cover_title), para(data["subtitle"], body),
             Spacer(1, 15), para(f"v{version}  /  {data['language']}  /  2026-11-01", sub),
             Spacer(1, 12), Workflow(data["workflow"], inner), Spacer(1, 18),
             para(data["scope"], body), Spacer(1, 12), para(data["intro"], body),
             Spacer(1, 20), para(data["legal"], small), PageBreak(),
             Paragraph(f'<a name="contents"/>{escape(data["contents"])}', heading),
             para(data["contentsHelp"], body), Spacer(1, 7)]
    for index, chapter in enumerate(chapters):
        anchor = f"s{index + 1:02}"
        text = f'<link href="#{anchor}" color="#243247">{escape(chapter["title"])}</link>'
        row = Table([[para(f"{index + 1:02}", number_style), Paragraph(text, body), para(str(index + 3), small)]],
                    colWidths=[31, inner - 61, 30])
        row.setStyle(TableStyle([("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
                                 ("LEFTPADDING", (0, 0), (-1, -1), 0),
                                 ("BOTTOMPADDING", (0, 0), (-1, -1), 2)]))
        story.append(row)
    story.extend([Spacer(1, 12), para(data["reading"], small)])
    ui = labels(language)
    for index, chapter in enumerate(chapters):
        story.extend([PageBreak(), para(f"{index + 1:02}  /  {data['chapter']}", sub),
                      Paragraph(f'<a name="s{index + 1:02}"/>{escape(chapter["title"])}', heading)])
        screen = "  /  ".join(ui[key] for key in SCREENS[index] if key in ui)
        if screen:
            story.append(para(f"{data['screenLabel']}: {screen}", small))
        story.extend([para(chapter["lead"], body), para(data["stepsLabel"], sub)])
        for number, text in enumerate(chapter["steps"], 1):
            row = Table([[para(f"{number}.", number_style), para(text, step_style)]], colWidths=[22, inner - 22])
            row.setStyle(TableStyle([("VALIGN", (0, 0), (-1, -1), "TOP"),
                                     ("LEFTPADDING", (0, 0), (-1, -1), 0),
                                     ("TOPPADDING", (0, 0), (-1, -1), 0),
                                     ("BOTTOMPADDING", (0, 0), (-1, -1), 7)]))
            story.append(row)
        story.append(para(data["detailsLabel"], sub))
        for text in chapter["details"]:
            story.append(para(text, body))
        tip = Table([[para(data["tipLabel"], sub)], [para(chapter["tip"], body)]], colWidths=[inner])
        tip.setStyle(TableStyle([("BACKGROUND", (0, 0), (-1, -1), LIGHT),
                                 ("BOX", (0, 0), (-1, -1), 0.5, colors.HexColor("#d7e0ea")),
                                 ("LEFTPADDING", (0, 0), (-1, -1), 12),
                                 ("RIGHTPADDING", (0, 0), (-1, -1), 12),
                                 ("TOPPADDING", (0, 0), (-1, -1), 3),
                                 ("BOTTOMPADDING", (0, 0), (-1, -1), 3)]))
        story.extend([Spacer(1, 8), KeepTogether(tip)])
    doc.build(story, onFirstPage=page_chrome, onLaterPages=page_chrome)
    reader = PdfReader(output)
    if len(reader.pages) != page_total:
        raise ValueError(f"{language}: expected {page_total} pages, found {len(reader.pages)}")
    writer = PdfWriter(clone_from=output)
    from pypdf.generic import NameObject, TextStringObject
    writer.root_object[NameObject("/Lang")] = TextStringObject(language)
    writer.add_outline_item(data["contents"], 1)
    for index, chapter in enumerate(chapters):
        writer.add_outline_item(chapter["title"], index + 2)
    writer.add_metadata({"/Title": f"TMC v{version} - {data['title']}", "/Author": "The Multipurpose Companion"})
    with output.open("wb") as stream:
        writer.write(stream)
    return {"language": language, "file": str(output), "pages": page_total,
            "words": sum(len(p.extract_text().split()) for p in PdfReader(output).pages)}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "output" / "pdf")
    args = parser.parse_args()
    version = re.search(r'\[workspace.package\]\s*version = "([^"]+)"',
                        (ROOT / "Cargo.toml").read_text()).group(1)
    register_fonts()
    args.output.mkdir(parents=True, exist_ok=True)
    sources = {lang: json.loads((SOURCE / f"{lang}.json").read_text(encoding="utf-8")) for lang in OUTPUTS}
    expected = [chapter["id"] for chapter in sources["es"]["sections"]]
    for lang, data in sources.items():
        assert [chapter["id"] for chapter in data["sections"]] == expected, lang
        assert len(data["workflow"]) == 3
        assert all(len(c["steps"]) == 4 and len(c["details"]) == 2 for c in data["sections"]), lang
    for lang, data in sources.items():
        print(json.dumps(build(lang, data, args.output / OUTPUTS[lang], version), ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
