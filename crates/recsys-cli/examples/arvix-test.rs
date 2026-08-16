use recsys_core::Scraper;
use recsys_scraper::ArxivScraper;



fn main() {

    let mut arvStruct = ArxivScraper;
    let papers = arvStruct.scrape(&vec!["Mycelium".to_string(), "5-ht2a".to_string(), "ai".to_string()], 5).expect("poop fart");

    for p in papers {
        println!("{}\n", p.title);
        println!("{}", p.text);
    }

}