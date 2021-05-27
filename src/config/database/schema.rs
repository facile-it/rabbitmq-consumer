table! {
    queues {
        id -> Integer,
        prefetch_count -> Nullable<Integer>,
        queue_name -> Varchar,
        consumer_name -> Varchar,
        command -> Varchar,
        command_timeout -> Nullable<Unsigned<BigInt>>,
        base64 -> Bool,
        start_hour -> Nullable<Time>,
        end_hour -> Nullable<Time>,
        count -> Integer,
        nack_code -> Nullable<Integer>,
        retry_wait -> Unsigned<BigInt>,
        retry_mode -> Varchar,
        enabled -> Bool,
    }
}
