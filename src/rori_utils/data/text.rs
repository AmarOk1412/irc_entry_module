use regex::Regex;

pub struct RoriTextData {
    author: String,
    content: String,
    client: String,
}

impl RoriTextData {
    pub fn new(author: String, content: String, client: String) -> RoriTextData {
        RoriTextData {
            author: author.replace("\"", "\\\""),
            content: content.replace("\"", "\\\""),
            client: client.replace("\"", "\\\""),
        }
    }

    pub fn to_string(&self) -> String {
        format!("{{
            \"author\":\"{}\",
            \"content\":\"{}\",
            \"client\":\"{}\",
            \"type\":\"text\"
        }}",
                self.author,
                self.content,
                self.client)
    }

    #[allow(dead_code)]
    #[allow(unused_variables)]
    fn answer_condition(&self, cond: &String) -> bool {
        let re = Regex::new(cond).unwrap();
        re.is_match(&*self.content)
    }
}
