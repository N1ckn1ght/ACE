use std::collections::HashMap;

pub struct Opening {
    pub book: HashMap<u64, Vec<u32>>,   // is empty if not initialized
    pub to_init: Vec<Page>              // is empty if initialized
}

impl Default for Opening {
    fn default() -> Opening {
        let mut pre = Vec::new();

        Self {
            book: HashMap::default(),
            to_init: pre
        }
    }
}

impl Opening {
    fn init(&mut self) {
        
    }
    
    fn is_initialized(&self) -> bool {
        self.to_init.is_empty() && !self.book.is_empty()
    }
}

struct Page {
    pub position: String,
    pub variations: Vec<u32>
}