use influxdb::Client as influx;
use influxdb::InfluxDbWriteable;

use chrono::{DateTime, Utc};

use tokio_postgres::NoTls;


struct Config {
    pg_database_host: String,
    pg_database_user: String,
    pg_database_pass: String,
    pg_database_name: String,
    influx_host:      String,
    influx_name:      String,
    influx_port:      String,
}


#[derive(Debug)]
#[derive(Clone)]
#[derive(InfluxDbWriteable)]
struct LemmyStats {
    time:                 DateTime<Utc>,
    registered_users:     i64,
    verified_users:       i64,
    unverified_users:     i64,
    approved_users:       i64,
    unapproved_users:     i64,
    num_of_apps:          i64,
    denied_users:         i64,
    known_communities:    i64,
    known_instances:      i64,
    known_comments:       i64,
    known_posts:          i64,
    comments_from_local:  i64,
    posts_from_local:     i64,
    upvotes_from_local:   i64,
    downvotes_from_local: i64,
}

impl LemmyStats {
    pub fn new() -> LemmyStats {
        LemmyStats {
            time:                 chrono::offset::Utc::now(),
            registered_users:     0,
            verified_users:       0,
            unverified_users:     0,
            approved_users:       0,
            unapproved_users:     0,
            num_of_apps:          0,
            denied_users:         0,
            known_communities:    0,
            known_instances:      0,
            known_comments:       0,
            known_posts:          0,
            comments_from_local:  0,
            posts_from_local:     0,
            upvotes_from_local:   0,
            downvotes_from_local: 0,
        }
    }
}

async fn collect_lemmy_stats(config: &Config) -> Result<LemmyStats, tokio_postgres::Error> {

    let (client, connection) = tokio_postgres::connect(format!("host={} user={} password={} dbname={}",
                                                               config.pg_database_host, config.pg_database_user, config.pg_database_pass, config.pg_database_name)
                                                       .as_str(), NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("PSQL Connection Error: {}", e);
        }
    });

    let mut stats = LemmyStats::new();

    // Get total reg'd users
    match get_count_of_rows_for_table(&client, "local_user".to_string()).await {
        Ok(total) => { stats.registered_users = total },
        Err(_) => { eprintln!("Unable to query local_user total!"); }
    }

    // Get total verified users
    match get_count_of_rows_for_table_with_condition(&client, "local_user".to_string(), "email_verified = true".to_string()).await {
        Ok(total) => { stats.verified_users = total },
        Err(_) => { eprintln!("Unable to query local_user verification total!"); }
    }

    // Get total unverified users
    match get_count_of_rows_for_table_with_condition(&client, "local_user".to_string(), "email_verified = false".to_string()).await {
        Ok(total) => { stats.unverified_users = total },
        Err(_) => { eprintln!("Unable to query local_user non-verified total!"); }
    }

    // Get total approved users
    match get_count_of_rows_for_table_with_condition(&client, "local_user".to_string(), "accepted_application = true".to_string()).await {
        Ok(total) => { stats.approved_users = total },
        Err(_) => { eprintln!("Unable to query local_user approved total!"); }
    }
    
    // Get total unapproved users
    match get_count_of_rows_for_table_with_condition(&client, "local_user".to_string(), "accepted_application = false".to_string()).await {
        Ok(total) => { stats.unapproved_users = total },
        Err(_) => { eprintln!("Unable to query local_user unapproved total!"); }
    }

    // Get total pending applications
    match get_count_of_rows_for_table_with_condition(&client, "registration_application".to_string(), "admin_id = null".to_string()).await {
        Ok(total) => { stats.num_of_apps = total },
        Err(_) => { eprintln!("Unable to query registration_application pending total!"); }
    }

    // Get total denied applications
    match get_count_of_rows_for_table_with_condition(&client, "registration_application".to_string(), "deny_reason IS NOT NULL".to_string()).await {
        Ok(total) => { stats.denied_users = total },
        Err(_) => { eprintln!("Unable to query registration_application pending total!"); }
    }
    
    // Get total known communities
    match get_count_of_rows_for_table(&client, "community".to_string()).await {
        Ok(total) => { stats.known_communities = total },
        Err(_) => { eprintln!("Unable to query communities total!"); }
    }
    
    // Get total known instances
    match get_count_of_rows_for_table(&client, "instance".to_string()).await {
        Ok(total) => { stats.known_instances = total },
        Err(_) => { eprintln!("Unable to query instances total!"); }
    }

    // Get total known comments
    match get_count_of_rows_for_table(&client, "comment".to_string()).await {
        Ok(total) => { stats.known_comments = total },
        Err(_) => { eprintln!("Unable to query comments total!"); }
    }

    // Get total known posts
    match get_count_of_rows_for_table(&client, "post".to_string()).await {
        Ok(total) => { stats.known_posts = total },
        Err(_) => { eprintln!("Unable to query posts total!"); }
    }

    // Get total number of comments made by people on our instance
    match get_count_of_rows_with_custom_statement(&client, "SELECT count(1) 
                                                  FROM comment INNER JOIN local_user ON creator_id = local_user.person_id").await {
        Ok(total) => { stats.comments_from_local = total },
        Err(_) => { eprintln!("Unable to query local comment total!"); }
    }

    // Get total number of posts made by people on our instance
    match get_count_of_rows_with_custom_statement(&client, "SELECT count(1) 
                                                  FROM post 
                                                  INNER JOIN local_user ON creator_id = local_user.person_id").await {
        Ok(total) => { stats.posts_from_local = total },
        Err(_) => { eprintln!("Unable to query local post total!"); }
    }

    // Get total number of upvotes made by people on our instance
    match get_count_of_rows_with_custom_statement(&client, "SELECT count(1) 
                                                  FROM comment_like 
                                                  INNER JOIN local_user ON comment_like.person_id = local_user.person_id 
                                                  WHERE score = 1;").await {
        Ok(total) => { stats.upvotes_from_local = total },
        Err(_) => { eprintln!("Unable to query local score (+1/upvote) total!"); }
    }

    // Get total number of downvotes made by people on our instance
    match get_count_of_rows_with_custom_statement(&client, "SELECT count(1) 
                                                  FROM comment_like 
                                                  INNER JOIN local_user ON comment_like.person_id = local_user.person_id 
                                                  WHERE score = -1;").await {
        Ok(total) => { stats.downvotes_from_local = total },
        Err(_) => { eprintln!("Unable to query local score (-1/upvote) total!"); }
    }

    return Ok(stats);

}

async fn get_count_of_rows_for_table(client: &tokio_postgres::Client, table_name: String) -> Result<i64, tokio_postgres::Error> {
    let query_string = format!("SELECT count(1) FROM {}", table_name);
    let query_str = query_string.as_str();
    let query = client.query(query_str, &[]);

    for row in query.await? {
        return Ok(row.get(0));
    }

    Ok(0)
}

async fn get_count_of_rows_for_table_with_condition(client: &tokio_postgres::Client, table_name: String, condition: String) -> Result<i64, tokio_postgres::Error> {
    let query_string = format!("SELECT count(1) FROM {} WHERE {}", table_name, condition);
    let query_str = query_string.as_str();
    let query = client.query(query_str, &[]);

    for row in query.await? {
        return Ok(row.get(0));
    }

    Ok(0)
}

async fn get_count_of_rows_with_custom_statement(client: &tokio_postgres::Client, statement: &str) -> Result<i64, tokio_postgres::Error> {
    let query = client.query(statement, &[]);

    for row in query.await? {
        return Ok(row.get(0));
    }

    Ok(0)
}

async fn push_lemmy_stats(stats: &LemmyStats, config: &Config) -> Result<(), influxdb::Error> {
    let client = influx::new(format!("http://{}:{}", config.influx_host, config.influx_port).as_str(), config.influx_name.clone());

    let instance_stats = vec!(
        stats.clone().into_query("stats")
    );

    match client.query(instance_stats).await {
        Ok(res) => println!("Wrote to Influx: {}", res),
        Err(err) => eprintln!("Unable to write to Influx: {}", err)
    }


    println!("Done pushing Influx stats");

    Ok(())
}

fn build_and_verify_config() -> Config {
    let pg_database_host = std::env::var("PG_DB_HOST").expect("A PG_DB_HOST env var is required!");
    let pg_database_user = std::env::var("PG_DB_USER").expect("A PG_DB_USER env var is required!"); 
    let pg_database_name = std::env::var("PG_DB_NAME").expect("A PG_DB_NAME env var is required!");
    let pg_database_pass = std::env::var("PG_DB_PASS").expect("A PG_DB_PASS env var is required!");
    let influx_host      = std::env::var("INFLUX_HOST").expect("An INFLUX_HOST env var is required!");
    let influx_name      = std::env::var("INFLUX_NAME").expect("An INFLUX_NAME env var is required!");
    let influx_port      = std::env::var("INFLUX_PORT").expect("An INFLUX_PORT env var is required!");
    Config {
        pg_database_host,
        pg_database_user,
        pg_database_name,
        pg_database_pass,
        influx_host,
        influx_name,
        influx_port
    }

}

#[tokio::main]
async fn main() -> Result<(), tokio_postgres::Error> {
    let config = build_and_verify_config();
    println!("Connecting to Postgres...");

    let stats = collect_lemmy_stats(&config).await;

    match stats {
        Ok(stats) => { 
            println!("Found stats: {:#?}", stats);
            match push_lemmy_stats(&stats, &config).await {
                Err(_) => eprintln!("Failed to push to influx!"),
                Ok(_) => { println!("Pushed stats to influx") }
            }
        },
        Err(err) => println!("Failed to grab Lemmy stats from postgres: {}", err)
    }

    Ok(())
}
