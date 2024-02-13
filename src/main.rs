use std::{collections::HashMap, time::Instant};
use reqwest;
use tokio::{self}; 
use scraper::{Html, Selector};
use rusqlite::{Connection, Result};
use rusqlite::params;

#[derive(Debug)]
struct Book {
    title: String,
    price: String,
    link: String,
    description: String,
}

async fn fetch(url : &str) -> Result<String, reqwest::Error>{
    // Send a GET request to the URL and await the response 
    let res = reqwest::get(url).await?;
    match res.status() {
        reqwest::StatusCode::OK => println!("Request was successful"),
        _ => println!("Request failed with status: {}", res.status()),
    }

    println!("Response Status: {}", res.status());
    let body = res.text().await?;
    //println!("Response Body {}", body);
    Ok(body)
}

async fn scrape_product_description(url : &str) -> Result<HashMap<String, String>, reqwest::Error> {
            let mut info = std::collections::HashMap::new();
            let res = reqwest::get(url).await?;
            let body = res.text().await?;
            let fragment = Html::parse_document(&body);

            // scrape book description
            let description = fragment
                .select(&Selector::parse("article.product_page > p").unwrap())
                .next()
                .unwrap()
                .inner_html();

            // scrape product information (table)
            let product_information_ = fragment
                .select(&Selector::parse("table.table.table-striped").unwrap())
                .next()
                .unwrap()
                .inner_html();

            let product_information = product_information_
            .replace("<tr>", "")
            .replace("</tr>", "")
            .replace("<tbody>", "")
            .replace("</tbody>", "")
            .replace("<th>", "")
            .replace("</th>", "")
            .replace("<td>", "")
            .replace("</td>", "")
            .replace("\n", "")
            .replace("Availability", ",Availability:")
            .replace("Price", ",Price:")
            .replace("Category", ",Category:")
            .replace("Number of reviews", ",Number of reviews:")
            .replace("UPC", ",UPC:")
            .replace("Product Type", ",Product Type:")
            .replace("Price (excl. tax)", ",Price (excl. tax):")
            .replace("Price (incl. tax)", ",Price (incl. tax):")
            .replace("Tax", ",Tax:")
            .replace("Availability", ",Availability");

            let parts: Vec<&str> = product_information.split(',').collect();
            for part in parts {
                let mut key_value: Vec<&str> = part.split(':').collect();
                
                for i in 0..key_value.len() {
                    key_value[i] = key_value[i].trim();
                }
                for i in 0..key_value.len() {
                    if key_value[i].len() > 0{
                        // print!("Key Value: {:?}", key_value);
                        let key = key_value[0];
                        let value = key_value[1];
                        // println!("Key: {}, Value: {}", key, value);
                        info.insert(key, value);        
                    }
                }
            }

        // println!("Product Information: {:?}", info);

        let mut data: HashMap<String, String> = HashMap::new();
        data.insert("Description".to_string(), description.to_string());
        for (key, value) in info {
            data.insert(key.to_string(), value.to_string());
        }

        Ok(data)
    }

async fn parse(html: &str) -> Vec<String> {
    let fragment = Html::parse_document(html);
    
    let mut product_details = Vec::new();

    for element in fragment.select(&Selector::parse("article.product_pod").unwrap()) {
        // scrape book titles
        let title = element
            .select(&Selector::parse("h3 > a").unwrap())
            .next()
            .unwrap()
            .inner_html();

        // scrape book links
        let links = element
            .select(&Selector::parse("h3 > a").unwrap())
            .next()
            .unwrap()
            .value()
            .attr("href")
            .unwrap();
        let link = format!("https://books.toscrape.com/{}", links);
        println!("Link: {}", link);

        // scrape product description
        let description =  scrape_product_description(&link).await.unwrap();

        // scrape book price
        let price = element
            .select(&Selector::parse("div.product_price > p.price_color").unwrap())
            .next()
            .unwrap()
            .inner_html();

        product_details.push((title, price, links, description));
    }
    //println!("Product Details: {:?}", product_details);
    product_details.iter().map(|(title, price,  links, description)| format!("Title: {}, Price: {},  Link: {}, Description: {:?}", title, price, links, description)).collect()
}

fn save_to_db(conn: &Connection, data: Vec<String>) -> Result<()> {
    for product in data {
        let parts: Vec<&str> = product.split(',').collect();
        let title = parts[0].split(':').collect::<Vec<&str>>()[1].trim();
        let price = parts[1].split(':').collect::<Vec<&str>>()[1].trim();
        let link = parts[2].split(':').collect::<Vec<&str>>()[1].trim();
        let description = parts[3].split(':').collect::<Vec<&str>>()[1].trim();

        conn.execute(
            "INSERT INTO books (title, price, link, description) VALUES (?1, ?2, ?3, ?4)",
            params![title, price, link, description],
        )?;
    }
    Ok(())
}

fn connect_to_db(filename: String) -> Result<Connection> {
    let conn = Connection::open(filename)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS books (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL,
            price TEXT NOT NULL,
            link TEXT NOT NULL,
            description TEXT NOT NULL
        )",
        params![],
    )?;
    Ok(conn)
}

fn read_from_db(conn: &Connection) -> Result<Vec<Book>> {
    let mut stmt = conn.prepare("SELECT title, price, link, description FROM books LIMIT 5")?;
    let book_iter = stmt.query_map(params![], |row| {
        Ok(Book {
            title: row.get(0)?,
            price: row.get(1)?,
            link: row.get(2)?,
            description: row.get(3)?,
        })
    })?;

    let mut books = Vec::new();
    for book in book_iter {
        books.push(book?);
    }
    Ok(books)
}

#[tokio::main]
async fn main() {
    let url = "https://books.toscrape.com/";
    println!("Fetching url: {}", url);

    // Use Instant from std::time
    let start = Instant::now(); 
    let data = fetch(url);
    let data = data.await.unwrap(); 
    //println!("Data: {:?}", data);
    
    let products = parse(&data).await;
    println!("Products: {:?}", products[0]);

    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);

    // connect to the database and save the data
    let conn = connect_to_db("books.db".to_string()).unwrap();
    save_to_db(&conn, products).unwrap();

    // read from the database
    let books = read_from_db(&conn).unwrap();
    println!("Books: {:?}", books);

    // close the database connection
    conn.close().unwrap();
}

