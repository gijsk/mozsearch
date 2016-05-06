use std::fs::File;
use std::env;
use std::io::BufReader;
use std::io::BufRead;
use std::io::Write;
use std::collections::HashMap;
use std::collections::BTreeMap;

extern crate tools;
use tools::find_source_file;
use tools::analysis::{read_analysis, read_target, AnalysisKind};

extern crate rustc_serialize;
use rustc_serialize::json::{Json, ToJson};

#[derive(Debug, RustcEncodable, RustcDecodable)]
struct SearchResult {
    lineno: u32,
    line: String,
}

impl ToJson for SearchResult {
    fn to_json(&self) -> Json {
        let mut obj = BTreeMap::new();
        obj.insert("lno".to_string(), self.lineno.to_json());
        obj.insert("line".to_string(), self.line.to_json());
        Json::Object(obj)
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();

    let tree_root = &args[1];
    let index_root = &args[2];
    //let mozsearch_root = &args[3];
    let objdir = &args[4];
    let filenames_file = &args[5];

    let output_file = index_root.to_string() + "/crossref";
    let jump_file = index_root.to_string() + "/jumps";

    let mut table = HashMap::new();
    let mut pretty_table = HashMap::new();
    let mut jumps = Vec::new();

    {
        let mut process_file = |path: &str| {
            print!("File {}\n", path);

            let analysis_fname = format!("{}/analysis/{}", index_root, path);
            let analysis = read_analysis(&analysis_fname, &read_target);

            let source_fname = find_source_file(path, tree_root, objdir);
            let source_file = match File::open(source_fname) {
                Ok(f) => f,
                Err(_) => {
                    println!("Unable to open source file");
                    return;
                },
            };
            let reader = BufReader::new(&source_file);
            let mut lines = Vec::new();
            for line in reader.lines() {
                match line {
                    Ok(l) => lines.push(l),
                    Err(_) => lines.push("".to_string()),
                }
            }

            for datum in analysis {
                for piece in datum.data {
                    let t1 = table.entry(piece.sym.to_owned()).or_insert(BTreeMap::new());
                    let t2 = t1.entry(piece.kind).or_insert(BTreeMap::new());
                    let t3 = t2.entry(path.to_owned()).or_insert(Vec::new());
                    let lineno = (datum.loc.lineno - 1) as usize;
                    if lineno >= lines.len() {
                        print!("Bad line number in file {} (line {})\n", path, lineno);
                        return;
                    }
                    let line = lines[lineno].clone();
                    let line_cut = line.trim();
                    let mut buf = String::new();
                    let mut i = 0;
                    for c in line_cut.chars() {
                        buf.push(c);
                        i += 1;
                        if i > 100 {
                            break;
                        }
                    }
                    t3.push(SearchResult { lineno: datum.loc.lineno, line: buf });

                    pretty_table.insert(piece.sym.to_owned(), piece.pretty.to_owned());
                }
            }
        };

        let f = File::open(filenames_file).unwrap();
        let file = BufReader::new(&f);
        for line in file.lines() {
            process_file(&line.unwrap());
        }
    }

    let mut outputf = File::create(output_file).unwrap();

    for (id, id_data) in table {
        let mut kindmap = BTreeMap::new();
        for (kind, kind_data) in &id_data {
            let mut result = Vec::new();
            for (path, results) in kind_data {
                let mut obj = BTreeMap::new();
                obj.insert("path".to_string(), path.to_json());
                obj.insert("lines".to_string(), results.to_json());
                result.push(Json::Object(obj));
            }
            let kindstr = match *kind {
                AnalysisKind::Use => "Uses",
                AnalysisKind::Def => "Definitions",
                AnalysisKind::Assign => "Assignments",
                AnalysisKind::Decl => "Declarations",
                AnalysisKind::Idl => "IDL",
            };
            kindmap.insert(kindstr.to_string(), Json::Array(result));
        }
        let kindmap = Json::Object(kindmap);

        let _ = outputf.write_all(format!("{}\n{}\n", id, kindmap.to_string()).as_bytes());

        if id_data.contains_key(&AnalysisKind::Def) {
            let defs = id_data.get(&AnalysisKind::Def).unwrap();
            if defs.len() == 1 {
                for (path, results) in defs {
                    if results.len() == 1 {
                        let mut v = Vec::new();
                        v.push(id.to_json());
                        v.push(path.to_json());
                        v.push(results[0].lineno.to_json());
                        let pretty = pretty_table.get(&id).unwrap();
                        v.push(pretty.to_json());
                        jumps.push(Json::Array(v));
                    }
                }
            }
        }
    }

    let mut jumpf = File::create(jump_file).unwrap();
    for jump in jumps {
        let _ = jumpf.write_all((jump.to_string() + "\n").as_bytes());
    }
}