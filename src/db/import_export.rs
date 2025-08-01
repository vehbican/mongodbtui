use futures::stream::TryStreamExt;
use mongodb::{
    Client,
    bson::{Document, doc},
};
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader, Write},
};
use tokio::fs::{self};

pub async fn export_collection(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);

    let mut cursor = collection.find(doc! {}).await?;

    let path = std::path::Path::new(file_path);

    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let mut file = File::create(path)?;

    while let Some(doc) = cursor.try_next().await? {
        let json_str = serde_json::to_string(&doc)?;
        writeln!(file, "{}", json_str)?;
    }

    Ok(())
}

pub async fn export_database(
    client: &Client,
    db_name: &str,
    folder_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::fs::create_dir_all(folder_path).await?;

    let db = client.database(db_name);
    let collections = db.list_collection_names().await?;

    for collection_name in collections {
        let file_path = format!("{}/{}.json", folder_path, collection_name);
        let path = std::path::Path::new(&file_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        super::import_export::export_collection(client, db_name, &collection_name, &file_path)
            .await?;
    }

    Ok(())
}
pub async fn import_collection(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);

    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut docs = vec![];

    for line in reader.lines() {
        let line = line?;
        let doc: Document = serde_json::from_str(&line)?;
        docs.push(doc);
    }

    if !docs.is_empty() {
        collection.insert_many(docs).await?;
    }

    Ok(())
}
pub async fn import_database(
    client: &Client,
    db_name: &str,
    folder_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(folder_path).await?;

    tokio::pin!(entries);

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(OsStr::to_str) {
                if file_name.ends_with(".json") {
                    let collection_name = file_name.trim_end_matches(".json");
                    super::import_export::import_collection(
                        client,
                        db_name,
                        collection_name,
                        path.to_str().unwrap(),
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}
