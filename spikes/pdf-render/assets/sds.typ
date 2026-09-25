#let d = json(bytes(sys.inputs.data))
#set document(title: d.product_name, author: d.supplier)
#set text(font: "Noto Sans", size: 9pt, lang: d.lang)
#set page(
  paper: "a4",
  margin: (top: 28mm, bottom: 20mm, x: 18mm),
  header: context [
    #set text(size: 8pt)
    #grid(columns: (1fr, auto), [*#d.product_name* \ #d.labels.sds_number: #d.sds_number], align(right)[#d.labels.version: #d.version \ #d.labels.revision_date: #d.revision_date])
    #line(length: 100%, stroke: 0.5pt)
  ],
  footer: context [
    #set text(size: 8pt)
    #align(center)[#d.labels.page #counter(page).display() #d.labels.of #counter(page).final().first()]
  ],
  background: if d.draft { rotate(-45deg, text(size: 90pt, fill: rgb(220, 0, 0, 40))[DRAFT]) },
)
#for s in d.sections [
  #block(fill: rgb("#e8eef5"), inset: 4pt, width: 100%)[*#s.number #s.title*]
  #for p in s.paragraphs [#p \ ]
  #if "pictograms" in s [
    #stack(dir: ltr, spacing: 4mm, ..s.pictograms.map(p => image("/assets/" + p + ".svg", width: 18mm)))
  ]
  #if "table" in s [
    #table(columns: (2fr, 1fr, 1fr, 3fr), stroke: 0.4pt, table.header(..s.table.header.map(h => [*#h*])), ..s.table.rows.flatten())
  ]
]
#if d.end_marker != none [#align(center)[— #d.end_marker —]]
