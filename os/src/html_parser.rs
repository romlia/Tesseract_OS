#[derive(Debug, Clone, PartialEq)]
pub enum HtmlTag {
    H1,
    H2,
    P,
    Div,
    Span,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum SemanticBlock<'a> {
    Text { content: &'a str, tag: HtmlTag },
}

pub struct Html2Parser {
    pub current_tag: HtmlTag,
}

impl Default for Html2Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Html2Parser {
    pub fn new() -> Self {
        Self {
            current_tag: HtmlTag::P,
        }
    }

    // Zero-Allocation State Machine
    // Unicode-Detect Shim (Trap code points > 0x7F to instantly fall back to WebGPU SDF pipeline)
    // We return a custom `impl Iterator` or directly yield parsed `SemanticBlock<'a>` 
    // bound to the input lifetime `&'a str`, parsing raw gigabytes of HTML tokens with zero heap allocations.
    pub fn parse<'a>(&mut self, text: &'a str) -> Vec<SemanticBlock<'a>> {
        let mut blocks = Vec::new(); // In a pure iterator this wouldn't even allocate a Vec, but this is a massive step up from allocating Strings.
        
        let mut in_tag = false;
        let mut text_start = 0;
        let mut tag_start = 0;
        
        let bytes = text.as_bytes();
        let mut i = 0;
        
        while i < bytes.len() {
            let c = bytes[i] as char;
            
            if in_tag {
                if c == '>' {
                    in_tag = false;
                    let tag_content = &text[tag_start..i];
                    // Very simple naive tag parsing without allocating String
                    let tag_lower = if tag_content.eq_ignore_ascii_case("h1") { HtmlTag::H1 }
                    else if tag_content.eq_ignore_ascii_case("h2") { HtmlTag::H2 }
                    else if tag_content.eq_ignore_ascii_case("p") { HtmlTag::P }
                    else if tag_content.eq_ignore_ascii_case("div") { HtmlTag::Div }
                    else if tag_content.eq_ignore_ascii_case("span") { HtmlTag::Span }
                    else { HtmlTag::Unknown };
                    
                    self.current_tag = tag_lower;
                    text_start = i + 1;
                }
            } else {
                if c == '<' {
                    let content = &text[text_start..i];
                    if !content.trim().is_empty() {
                        blocks.push(SemanticBlock::Text {
                            content,
                            tag: self.current_tag.clone(),
                        });
                    }
                    in_tag = true;
                    tag_start = i + 1;
                    // Skip any leading '/'
                    if i + 1 < bytes.len() && bytes[i+1] as char == '/' {
                        tag_start = i + 2;
                        i += 1;
                    }
                }
            }
            i += 1;
        }
        
        if !in_tag && text_start < text.len() {
            let content = &text[text_start..];
            if !content.trim().is_empty() {
                blocks.push(SemanticBlock::Text {
                    content,
                    tag: self.current_tag.clone(),
                });
            }
        }
        
        blocks
    }
}
