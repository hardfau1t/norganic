use handlebars::Handlebars;
use miette::{Context, IntoDiagnostic};
use serde::Serialize;
use tracing::{debug, instrument, warn};

mod basic;
mod heading;
mod link;
mod paragraph;

pub fn parse_and_render_body<'h>(input: &str, hbr: &Handlebars<'h>) -> miette::Result<String> {
    let tokens = norg::parse_tree(&input).map_err(|e| miette::miette!("failed to parse: {e:?}"))?;
    debug!("found tokens: {tokens:#?}");
    tokens.into_iter().map(|ast| render_ast(ast, hbr)).collect()
}

pub async fn dump_ast(path: std::path::PathBuf) -> miette::Result<()> {
    let input = tokio::fs::read_to_string(&path)
        .await
        .into_diagnostic()
        .wrap_err_with(|| format!("Couldn't read {path:?}"))?;
    let tokens = norg::parse(&input).map_err(|e| miette::miette!("failed to parse: {e:?}"))?;
    println!("{tokens:#?}");
    Ok(())
}

#[derive(Serialize, Debug)]
struct Para {
    para: String,
}

#[instrument(skip(hbr))]
fn render_ast(ast: norg::NorgAST, hbr: &Handlebars) -> miette::Result<String> {
    let mut rendered_string = String::new();
    match ast {
        norg::NorgAST::Paragraph(p) => {
            let mut para = String::new();
            p.into_iter()
                .map(|segment| paragraph::render_paragraph(segment, &mut para, hbr))
                .collect::<Result<(), _>>()
                .into_diagnostic()
                .wrap_err("Failed to construct paragraph")?;
            let para = Para { para };
            let rendered_para = hbr
                .render("paragraph", &para)
                .into_diagnostic()
                .wrap_err("Failed to render paragraph")?;
            rendered_string.push_str(&rendered_para);
        }
        //norg::NorgASTFlat::NestableDetachedModifier {
        //    modifier_type,
        //    level,
        //    extensions,
        //    content,
        //} => todo!(),
        //norg::NorgASTFlat::RangeableDetachedModifier { modifier_type, title, extensions, content } => todo!(),
        norg::NorgAST::Heading {
            level,
            title,
            extensions,
            content,
        } => {
            let rendered_content = content
                .into_iter()
                .map(|content_ast| render_ast(content_ast, hbr))
                .collect::<Result<_, _>>()?;
            heading::render_heading(
                level,
                title,
                extensions,
                rendered_content,
                &mut rendered_string,
                hbr,
            )
            .into_diagnostic()
            .wrap_err("Failed to construct paragraph")?;
        }
        //norg::NorgASTFlat::CarryoverTag { tag_type, name, parameters, next_object } => todo!(),
        //norg::NorgASTFlat::VerbatimRangedTag { name, parameters, content } => todo!(),
        //norg::NorgASTFlat::RangedTag { name, parameters, content } => todo!(),
        //norg::NorgASTFlat::InfirmTag { name, parameters } => todo!(),
        _ => {
            warn!("Rendering is not implemented for this item");
        }
    };
    Ok(rendered_string)
}

/// register all the helpers from submodule
pub fn registser_helpers(hbr: &mut handlebars::Handlebars) {
    heading::registser_helpers(hbr);
}

#[cfg(test)]
mod tests {
    use super::parse_and_render_body;

    #[test]
    fn link_to_norg_file() {
        let content = "{:abc/def:}[link to def]";
        let expected = "<p><a href=\"abc/def.norg\">link to def</a></p>";
        let mut hbr = handlebars::Handlebars::new();
        let load_options = handlebars::DirectorySourceOptions::default();
        hbr.register_templates_directory("./templates", load_options)
            .expect("couldn't load handlebars");
        assert_eq!(
            parse_and_render_body(content, &hbr).expect("couldn't parse content"),
            expected,
            "html with a paragraph pointing to another norg file"
        );
    }
}
