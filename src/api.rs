use std::string::String;

struct BlueRideUser {
    name: String,
    email: String,
    phone_number: String,
}

enum NotificationChannel {
    Email,
    APN,
}

enum NotificationPurpose {
    Matched {
        match_id: String,
        group: Vec<BlueRideUser>,
        datetime_start: String,
        datetime_end: String,
    },

    Canceled {
        match_id: String,
        group: Vec<BlueRideUser>,
        datetime_start: String,
        datetime_end: String,
        reason: String,
    },
}

struct BlueRideNotification {
    target_user: BlueRideUser,
    channels: Vec<NotificationChannel>,
    payload: NotificationPurpose,
}

fn handle_queue_request() {}
