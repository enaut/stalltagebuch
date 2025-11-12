use crate::error::AppError;
use crate::models::EggRecord;
use rusqlite::Connection;

/// Statistik-Daten für Eier-Produktion
#[derive(Debug, Clone)]
pub struct EggStatistics {
    pub total_records: i32,
    pub total_eggs: i32,
    pub daily_average: f64,
    pub weekly_average: f64,
    pub monthly_average: f64,
    pub min_eggs: i32,
    pub max_eggs: i32,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

/// Berechnet Statistiken für einen bestimmten Zeitraum
pub fn calculate_statistics(
    conn: &Connection,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<EggStatistics, AppError> {
    // Query für Zeitraum-Filter
    let (query, params) = if let (Some(start), Some(end)) = (start_date, end_date) {
        (
            "SELECT 
                COUNT(*) as count,
                SUM(total_eggs) as sum,
                AVG(total_eggs) as avg,
                MIN(total_eggs) as min,
                MAX(total_eggs) as max,
                MIN(record_date) as first_date,
                MAX(record_date) as last_date
             FROM egg_records 
             WHERE record_date BETWEEN ?1 AND ?2",
            vec![start, end],
        )
    } else if let Some(start) = start_date {
        (
            "SELECT 
                COUNT(*) as count,
                SUM(total_eggs) as sum,
                AVG(total_eggs) as avg,
                MIN(total_eggs) as min,
                MAX(total_eggs) as max,
                MIN(record_date) as first_date,
                MAX(record_date) as last_date
             FROM egg_records 
             WHERE record_date >= ?1",
            vec![start],
        )
    } else if let Some(end) = end_date {
        (
            "SELECT 
                COUNT(*) as count,
                SUM(total_eggs) as sum,
                AVG(total_eggs) as avg,
                MIN(total_eggs) as min,
                MAX(total_eggs) as max,
                MIN(record_date) as first_date,
                MAX(record_date) as last_date
             FROM egg_records 
             WHERE record_date <= ?1",
            vec![end],
        )
    } else {
        (
            "SELECT 
                COUNT(*) as count,
                SUM(total_eggs) as sum,
                AVG(total_eggs) as avg,
                MIN(total_eggs) as min,
                MAX(total_eggs) as max,
                MIN(record_date) as first_date,
                MAX(record_date) as last_date
             FROM egg_records",
            vec![],
        )
    };

    let mut stmt = conn.prepare(query)?;
    let result = stmt.query_row(rusqlite::params_from_iter(params.iter()), |row| {
        let count: i32 = row.get(0)?;
        let sum: Option<i32> = row.get(1)?;
        let avg: Option<f64> = row.get(2)?;
        let min: Option<i32> = row.get(3)?;
        let max: Option<i32> = row.get(4)?;
        let first: Option<String> = row.get(5)?;
        let last: Option<String> = row.get(6)?;

        Ok((count, sum, avg, min, max, first, last))
    })?;

    let (
        total_records,
        total_eggs_opt,
        daily_avg,
        min_eggs_opt,
        max_eggs_opt,
        first_date,
        last_date,
    ) = result;

    let total_eggs = total_eggs_opt.unwrap_or(0);
    let daily_average = daily_avg.unwrap_or(0.0);
    let min_eggs = min_eggs_opt.unwrap_or(0);
    let max_eggs = max_eggs_opt.unwrap_or(0);

    // Berechne Wochen- und Monatsdurchschnitt
    let weekly_average = if total_records >= 7 {
        calculate_weekly_average(conn, start_date, end_date)?
    } else {
        daily_average
    };

    let monthly_average = if total_records >= 30 {
        calculate_monthly_average(conn, start_date, end_date)?
    } else {
        daily_average
    };

    Ok(EggStatistics {
        total_records,
        total_eggs,
        daily_average,
        weekly_average,
        monthly_average,
        min_eggs,
        max_eggs,
        first_date,
        last_date,
    })
}

/// Berechnet 7-Tage gleitenden Durchschnitt
fn calculate_weekly_average(
    conn: &Connection,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<f64, AppError> {
    // Simplified: nehme die letzten 7 Tage
    let records = crate::services::list_egg_records(conn, start_date, end_date)?;

    if records.len() < 7 {
        return Ok(records.iter().map(|r| r.total_eggs as f64).sum::<f64>() / records.len() as f64);
    }

    // Nimm die letzten 7 Einträge
    let last_seven: Vec<&EggRecord> = records.iter().rev().take(7).collect();
    let sum: i32 = last_seven.iter().map(|r| r.total_eggs).sum();

    Ok(sum as f64 / 7.0)
}

/// Berechnet 30-Tage Durchschnitt
fn calculate_monthly_average(
    conn: &Connection,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<f64, AppError> {
    let records = crate::services::list_egg_records(conn, start_date, end_date)?;

    if records.len() < 30 {
        return Ok(records.iter().map(|r| r.total_eggs as f64).sum::<f64>() / records.len() as f64);
    }

    // Nimm die letzten 30 Einträge
    let last_thirty: Vec<&EggRecord> = records.iter().rev().take(30).collect();
    let sum: i32 = last_thirty.iter().map(|r| r.total_eggs).sum();

    Ok(sum as f64 / 30.0)
}

/// Gibt aggregierte Daten für die letzten N Tage zurück
pub fn get_recent_trend(conn: &Connection, days: i32) -> Result<Vec<(String, i32)>, AppError> {
    let records = crate::services::list_egg_records(conn, None, None)?;

    Ok(records
        .iter()
        .take(days as usize)
        .map(|r| (r.record_date.format("%Y-%m-%d").to_string(), r.total_eggs))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use crate::models::EggRecord;
    use chrono::NaiveDate;

    #[test]
    fn test_calculate_statistics_empty() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();

        let stats = calculate_statistics(&conn, None, None).unwrap();

        assert_eq!(stats.total_records, 0);
        assert_eq!(stats.total_eggs, 0);
        assert_eq!(stats.daily_average, 0.0);
    }

    #[test]
    fn test_calculate_statistics_with_data() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();

        // 5 Tage mit Daten
        for i in 1..=5 {
            let record = EggRecord::new(
                NaiveDate::from_ymd_opt(2025, 11, i as u32).unwrap(),
                10 + i as i32,
            );
            crate::services::add_egg_record(&conn, &record).unwrap();
        }

        let stats = calculate_statistics(&conn, None, None).unwrap();

        assert_eq!(stats.total_records, 5);
        assert_eq!(stats.total_eggs, 65); // 11+12+13+14+15
        assert_eq!(stats.min_eggs, 11);
        assert_eq!(stats.max_eggs, 15);
        assert!((stats.daily_average - 13.0).abs() < 0.01);
    }

    #[test]
    fn test_get_recent_trend() {
        let conn = Connection::open_in_memory().unwrap();
        database::schema::init_schema(&conn).unwrap();

        // 10 Tage hinzufügen
        for i in 1..=10 {
            let record = EggRecord::new(
                NaiveDate::from_ymd_opt(2025, 11, i as u32).unwrap(),
                (i * 2) as i32,
            );
            crate::services::add_egg_record(&conn, &record).unwrap();
        }

        let trend = get_recent_trend(&conn, 5).unwrap();

        assert_eq!(trend.len(), 5);
        assert_eq!(trend[0].1, 20); // Neuester Tag (10*2)
        assert_eq!(trend[4].1, 12); // 5 Tage zurück (6*2)
    }
}
