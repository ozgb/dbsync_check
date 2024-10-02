use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{collections::HashMap, io::Write};

use blockfrost::{BlockFrostSettings, BlockfrostAPI, BlockfrostResult, Pagination};

use clap::{Parser, Subcommand};

/// A tool that interacts with Blockfrost or Dbsync services.
#[derive(Parser)]
struct Cli {
    /// Subcommands available: blockfrost or dbsync
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Use the Blockfrost API
    Blockfrost {
        /// The Blockfrost project ID
        #[arg(env)]
        blockfrost_project_id: String,

        /// Start epoch
        #[arg(long)]
        epoch_start: i32,

        /// End epoch
        #[arg(long)]
        epoch_end: i32,
    },
    /// Use the Dbsync database
    Dbsync {
        /// The database URL
        #[arg(env)]
        database_url: String,

        /// Start epoch
        #[arg(long)]
        epoch_start: i32,

        /// End epoch
        #[arg(long)]
        epoch_end: i32,
    },
}

type PoolStake = HashMap<String, u128>;

fn build_api(project_id: &str) -> BlockfrostResult<BlockfrostAPI> {
    let settings = BlockFrostSettings::new();
    let api = BlockfrostAPI::new(project_id, settings);

    Ok(api)
}

async fn read_blockfrost_epoch(api: &BlockfrostAPI, epoch: i32) -> BlockfrostResult<PoolStake> {
    let pagination = Pagination::default();

    let epochs_stakes = api.epochs_stakes(epoch, pagination).await?;

    let mut map = HashMap::new();
    for stake in epochs_stakes.iter() {
        let stake_amount: u128 = stake.amount.parse().unwrap();
        match map.get_mut(&stake.pool_id) {
            Some(amount) => {
                *amount += stake_amount;
            }
            None => {
                map.insert(stake.pool_id.clone(), stake_amount);
            }
        }
    }

    Ok(map)
}

async fn read_dbsync_epoch(pool: Pool<Postgres>, epoch: i32) -> sqlx::Result<PoolStake> {
    let records = sqlx::query!(
        r#"
        SELECT ph.view as pool_hash, CAST(SUM(es.amount) as text) as "stake!: String"
        FROM epoch_stake es
        INNER JOIN pool_hash ph ON es.pool_id = ph.id
        WHERE es.epoch_no = $1
        GROUP BY ph.view
        "#,
        epoch
    )
    .fetch_all(&pool)
    .await?;

    let mut map = HashMap::new();
    for record in records {
        let stake_amount: u128 = record.stake.parse().unwrap();
        map.insert(record.pool_hash, stake_amount);
    }

    Ok(map)
}

fn write_csv(prefix: &str, epoch: i32, map: PoolStake) -> BlockfrostResult<()> {
    // Sort the map by pool
    let mut map = map.into_iter().collect::<Vec<_>>();
    map.sort_by(|a, b| a.0.cmp(&b.0));

    // Write file
    std::fs::create_dir_all("csv").expect("Could not create csv directory");

    let mut file = std::fs::File::create(format!("csv/{prefix}_{epoch}_stake.csv"))?;
    file.write_all("pool_id,stake\n".as_bytes())?;
    for (pool_id, stake) in map.iter() {
        file.write_all(format!("{pool_id},{stake}\n").as_bytes())?;
    }
    file.flush()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    match args.command {
        Commands::Blockfrost {
            blockfrost_project_id,
            epoch_start,
            epoch_end,
        } => {
            let api = build_api(&blockfrost_project_id)?;
            for epoch in epoch_start..=epoch_end {
                let map = read_blockfrost_epoch(&api, epoch).await?;
                write_csv("blockfrost", epoch, map)?;
            }
        }
        Commands::Dbsync {
            database_url,
            epoch_start,
            epoch_end,
        } => {
            let pool: Pool<Postgres> = PgPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await?;

            for epoch in epoch_start..=epoch_end {
                let map = read_dbsync_epoch(pool.clone(), epoch).await?;
                write_csv("dbsync", epoch, map)?;
            }
        }
    }

    Ok(())
}
