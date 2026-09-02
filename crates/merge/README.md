# quillmark-merge

Bulk document generation for [Quillmark](https://github.com/borb-sh/quillmark): a
`MergeSpec` interpreted over input rows (or over documents already in the
values form) into `Document`s, with a per-row report. Engine-free: the plan
hands documents to whatever render loop the surface runs, so a large batch
validates before any compilation is paid.

```rust
use quillmark_merge::{plan, Input, MergeSpec};

let spec = MergeSpec::from_yaml(r#"
$quill: certificate@1.2.0
map:
  recipient:  { column: Name }
  awarded_on: { column: Date, format: "%m/%d/%Y" }
  event:      { value: "Rustconf 2026" }
output: "{recipient}-certificate"
"#)?;
let plan = plan(&quill, &spec, &Input::Rows(rows));
for d in &plan.report {
    println!("{:?} {:?} {}", d.row, d.column, d.diagnostic.message);
}
if plan.is_clean() {
    for doc in &plan.documents {
        engine.render(&quill, &doc.document, &opts)?;
    }
}
```

The `quillmark` CLI's `merge` verb drives this crate over CSV, TSV and JSON.
The contract is `prose/canon/MERGE.md` in the repository.
