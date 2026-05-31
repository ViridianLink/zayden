1. Improve error varients (gambing, family)
2. Move hard coded values into bot config or database
3. Update destiny2 spreadsheet sync to a weekly cron job
4. Don't slient fail. Look for early returns (`return Ok(())`, `return ""` etc) that could be programmer error and instead return error.