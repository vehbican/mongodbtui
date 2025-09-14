use futures::stream::TryStreamExt;
use mongodb::{
    Client,
    bson::oid::ObjectId,
    bson::{Bson, Document, doc},
    error::Error,
    options::{ClientOptions, FindOptions},
};
use serde_json::from_str;
use std::time::Duration;

pub async fn connect_to_uri(uri: &str) -> Result<Client, mongodb::error::Error> {
    let mut options = ClientOptions::parse(uri).await?;
    options.app_name = Some("MongoTUI".to_string());
    options.connect_timeout = Some(Duration::from_secs(5));

    Client::with_options(options)
}

pub async fn list_databases(client: &Client) -> Result<Vec<String>, mongodb::error::Error> {
    client.list_database_names().await
}

pub async fn list_collections(client: &Client, db_name: &str) -> Result<Vec<String>, Error> {
    let db = client.database(db_name);
    let mut names = db.list_collection_names().await?;

    names.sort();
    Ok(names)
}

pub async fn fetch_documents(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    skip: u64,
    limit: u64,
    filter_text: &str,
    sort_text: &str,
) -> Result<Vec<Document>, Error> {
    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);

    let filter_doc: Document = if !filter_text.trim().is_empty() {
        match from_str::<Document>(filter_text) {
            Ok(doc) => doc,
            Err(_) => doc! {},
        }
    } else {
        doc! {}
    };

    let sort_doc: Document = if !sort_text.trim().is_empty() {
        match from_str::<Document>(sort_text) {
            Ok(doc) => doc,
            Err(_) => doc! {},
        }
    } else {
        doc! {}
    };

    let options = FindOptions::builder()
        .skip(Some(skip))
        .limit(Some(limit as i64))
        .sort(Some(sort_doc))
        .build();

    let mut cursor = collection.find(filter_doc).with_options(options).await?;

    let mut docs = Vec::new();
    while let Some(doc) = cursor.try_next().await? {
        docs.push(doc);
    }

    Ok(docs)
}
pub async fn rename_collection(
    client: &Client,
    db_name: &str,
    old_name: &str,
    new_name: &str,
) -> Result<(), mongodb::error::Error> {
    let admin_db = client.database("admin");

    let command = doc! {
        "renameCollection": format!("{}.{}", db_name, old_name),
        "to": format!("{}.{}", db_name, new_name),
        "dropTarget": false
    };

    admin_db.run_command(command).await.map(|_| ())
}
pub async fn count_documents(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    filter_text: &str,
) -> Result<u64, mongodb::error::Error> {
    use serde_json::from_str;

    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);

    let filter_doc = if !filter_text.trim().is_empty() {
        from_str::<Document>(filter_text).unwrap_or_default()
    } else {
        doc! {}
    };

    collection.count_documents(filter_doc).await
}
pub async fn apply_edited_document(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    original: &Document,
    edited: &Document,
) -> Result<(), Box<dyn std::error::Error>> {
    let id_bson = edited.get("_id").ok_or("_id alanı yok")?;
    let oid = match id_bson {
        Bson::ObjectId(oid) => *oid,
        _ => return Err("_id bir ObjectId olmalı".into()),
    };

    let (mut set_doc, unset_doc) = diff_docs_deep(original, edited);

    set_doc.remove("_id");

    let mut update = Document::new();
    if !set_doc.is_empty() {
        update.insert("$set", Bson::Document(set_doc));
    }
    if !unset_doc.is_empty() {
        update.insert("$unset", Bson::Document(unset_doc));
    }

    if update.is_empty() {
        return Ok(());
    }

    let db = client.database(db_name);
    let coll = db.collection::<Document>(collection_name);
    coll.update_one(doc! { "_id": oid }, update)
        .await
        .map(|_| ())
        .map_err(Into::into)
}

fn diff_docs_deep(original: &Document, edited: &Document) -> (Document, Document) {
    let mut set_doc = Document::new();
    let mut unset_doc = Document::new();

    for (k, new_v) in edited.iter() {
        if k == "_id" {
            continue;
        }
        match original.get(k) {
            None => {
                set_doc.insert(k, new_v.clone());
            }
            Some(old_v) => {
                diff_value("", k, old_v, new_v, &mut set_doc);
            }
        }
    }

    for (k, _) in original.iter() {
        if k == "_id" {
            continue;
        }
        if !edited.contains_key(k) {
            unset_doc.insert(k, Bson::String("".into()));
        }
    }

    (set_doc, unset_doc)
}

fn diff_value(prefix: &str, key: &str, old_v: &Bson, new_v: &Bson, set_doc: &mut Document) {
    use Bson::*;
    if old_v == new_v {
        return;
    }

    let full_key = if prefix.is_empty() {
        key.to_string()
    } else {
        format!("{prefix}.{key}")
    };

    match (old_v, new_v) {
        (Document(old_d), Document(new_d)) => {
            for (child_k, new_child_v) in new_d.iter() {
                match old_d.get(child_k) {
                    Some(old_child_v) => {
                        diff_value(&full_key, child_k, old_child_v, new_child_v, set_doc);
                    }
                    None => {
                        let dotted = format!("{full_key}.{child_k}");
                        set_doc.insert(dotted, new_child_v.clone());
                    }
                }
            }
        }
        (Array(_), Array(_)) => {
            set_doc.insert(full_key, new_v.clone());
        }
        _ => {
            set_doc.insert(full_key, new_v.clone());
        }
    }
}

pub async fn apply_edited_json(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    original: &Document,
    edited_json: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let edited_doc: Document = match serde_json::from_str(edited_json) {
        Ok(d) => d,
        Err(_) => {
            let v: serde_json::Value = serde_json::from_str(edited_json)?;
            match bson::to_bson(&v)? {
                Bson::Document(d) => d,
                _ => return Err("JSON bir obje ({}) olmalı".into()),
            }
        }
    };
    apply_edited_document(client, db_name, collection_name, original, &edited_doc).await
}
pub async fn delete_documents_by_filter(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    filter_text: &str,
) -> Result<(), mongodb::error::Error> {
    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);

    let filter_doc = if !filter_text.trim().is_empty() {
        serde_json::from_str::<Document>(filter_text).unwrap_or_else(|_| doc! {})
    } else {
        doc! {}
    };

    collection.delete_many(filter_doc).await.map(|_| ())
}
pub async fn delete_collection(
    client: &Client,
    db_name: &str,
    collection_name: &str,
) -> Result<(), mongodb::error::Error> {
    let db = client.database(db_name);
    db.collection::<Document>(collection_name).drop().await
}

pub async fn delete_field_in_document(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    document_id: ObjectId,
    field_name: &str,
) -> Result<(), mongodb::error::Error> {
    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);

    let filter = doc! { "_id": document_id };
    let update = doc! { "$unset": { field_name: "" } };

    collection.update_one(filter, update).await.map(|_| ())
}

pub async fn delete_document_by_id(
    client: &Client,
    db_name: &str,
    collection_name: &str,
    document_id: ObjectId,
) -> Result<(), mongodb::error::Error> {
    let db = client.database(db_name);
    let collection = db.collection::<Document>(collection_name);
    collection
        .delete_one(doc! { "_id": document_id })
        .await
        .map(|_| ())
}
pub async fn delete_database(
    client: &Client,
    db_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    client.database(db_name).drop().await?;
    Ok(())
}
