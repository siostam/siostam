use handlebars::Handlebars;
use serde_json::json;
use std::fs::File;
use std::io;
use std::io::{BufWriter, Write};
use log::info;

/// Heavy method which load the handlebars templates requires to generate .dot files
pub fn init_registry() -> Handlebars {
    let mut reg = Handlebars::new();

    reg.register_template_string("tpl_begin_graph", include_str!("templates/begin_graph.hbs"))
        .expect("Issue when registering tpl_begin_graph");
    reg.register_template_string("tpl_end_graph", include_str!("templates/end_graph.hbs"))
        .expect("Issue when registering tpl_end_graph");

    reg.register_template_string(
        "tpl_begin_cluster",
        include_str!("templates/begin_cluster.hbs"),
    )
    .expect("Issue when registering tpl_begin_cluster");
    reg.register_template_string("tpl_end_cluster", include_str!("templates/end_cluster.hbs"))
        .expect("Issue when registering tpl_end_cluster");

    reg.register_template_string("tpl_node", include_str!("templates/node.hbs"))
        .expect("Issue when registering tpl_node");
    reg.register_template_string("tpl_edge", include_str!("templates/edge.hbs"))
        .expect("Issue when registering tpl_edge");

    reg
}

/// The DotBuilder store the templates and the handle to the generated file
pub struct DotBuilder {
    reg: Handlebars,
    bufwriter: BufWriter<File>,
}

impl DotBuilder {
    /// Load handle bars, open-truncate or create the file and print the start of the graph.
    pub fn new(path: &str) -> io::Result<DotBuilder> {
        // Prepare the file and the renderer
        let file = File::create(path)?;
        let reg = init_registry();
        let mut bufwriter = BufWriter::new(file);

        // Write the beginning of the file
        reg.render_to_write("tpl_begin_graph", &(), &mut bufwriter)
            .expect("Error when rendering the beginning of file");

        Ok(DotBuilder { reg, bufwriter })
    }

    /// Print a new cluster in the file
    pub fn begin_cluster(&mut self, indent: &str, id: &str, name: &str) {
        let data = &json!({"indent": indent, "id": id, "name": name });
        self.reg
            .render_to_write("tpl_begin_cluster", data, &mut self.bufwriter)
            .expect("Error when rendering the beginning of the cluster");
    }

    /// Print the end of a cluster in the file
    pub fn end_cluster(&mut self, indent: &str) {
        let data = &json!({ "indent": indent });
        self.reg
            .render_to_write("tpl_end_cluster", data, &mut self.bufwriter)
            .expect("Error when rendering the end of the cluster");
    }

    /// Print a new node in the file
    pub fn add_node(&mut self, indent: &str, id: &str, name: &str) {
        let data = &json!({"indent": indent, "id": id, "name": name });
        self.reg
            .render_to_write("tpl_node", data, &mut self.bufwriter)
            .expect("Error when rendering the node");
    }

    /// Print a new edge in the file
    pub fn add_edge(&mut self, indent: &str, id_a: &str, id_b: &str) {
        let data = &json!({"indent": indent, "idA": id_a, "idB": id_b });
        self.reg
            .render_to_write("tpl_edge", data, &mut self.bufwriter)
            .expect("Error when rendering the edge");
    }

    /// Print the end of the file, flush and close the handle
    pub fn close(mut self) -> io::Result<()> {
        self.reg
            .render_to_write("tpl_end_graph", &(), &mut self.bufwriter)
            .expect("Error when rendering the end of file");
        self.bufwriter.flush()?;
        Ok(())
    }
}

/// Call to graphviz executable to create the SVG file
pub fn generate_file_from_dot(path: &str) {
    use std::process::Command;

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "fdp", "-Tsvg", path, "-O"])
            .output()
            .expect("failed to execute process")
    } else {
        Command::new("sh")
            .args(&["-c", "fdp", "-Tsvg", path, "-O"])
            .output()
            .expect("failed to execute process")
    };

    String::from_utf8_lossy(output.stdout.as_slice())
        .lines()
        .for_each(|l| info!("{}", l));
}
