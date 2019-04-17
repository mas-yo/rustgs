use futures::{prelude::*, sync::mpsc};
use std::{sync::Arc, sync::RwLock, thread};
// use diesel::prelude::*;
// use diesel::MysqlConnection;
// use diesel::connection::SimpleConnection;
use lazy_static::lazy_static;
use mysql;

pub type DBResult = mysql::Row;
pub type DBError = ();
pub type DBQuery = Vec<String>;

pub type DBResultSender = futures::sync::mpsc::Sender<DBResult>;
pub type DBResultReceiver = futures::sync::mpsc::Receiver<DBResult>;
pub type DBQuerySender = futures::sync::mpsc::Sender<(DBQuery, DBResultSender)>;

lazy_static! {
    pub(crate) static ref DB_SYNC: Arc<RwLock<mysql::Conn>> = {
        Arc::new(RwLock::new(mysql::Conn::new("mysql://rustgs:@localhost/rustgs").expect("db conn err")))
    };
}

pub(crate) fn sync_db() -> Arc<RwLock<mysql::Conn>> {
    DB_SYNC.clone()
}

pub fn start_database() -> (impl Future<Item = (), Error = ()>, DBQuerySender) {
    let mut mysql = mysql::Conn::new("mysql://rustgs:@localhost/rustgs").expect("db conn err");
    println!("db connected");
    let (query_tx, query_rx) = mpsc::channel::<(DBQuery, mpsc::Sender<DBResult>)>(24);

    let task = query_rx.for_each(move |(queries, result_tx)| {
        for query in queries {
            println!("DB QUERY: {}", query);
            for row in mysql.query(query).unwrap() {
                // let value:String = mysql::from_row(row.unwrap());
                // println!("selected value:{}", value);
                // let result = vec![value];
                let row = row.unwrap();
                let send_task = result_tx.clone().send(row).map(|_| ()).map_err(|_| ());
                tokio::spawn(send_task);
            }
        }

        Ok(())
    });
    (task, query_tx)
}

// struct ReceiveDBResult {
//     rx: DBResultReceiver,
// }
// impl Future for ReceiveDBResult {
//     type Item = ();
//     type Error = ();
//     fn poll(&mut self) -> Poll<(),()> {
//         self.rx.poll()
//     }
// }
// type ForEachDBResult = futures::stream::ForEach<Stream<Item=DBResult,Error=()>,FnMut(DBResult)->(),IntoFuture<>;

// type ForEachDBResult = <futures::sync::mpsc::Receiver<DBResult> as Stream<Item=DBResult,Error=()>>::ForEach;

pub(crate) trait NewQuery {
    fn new_query_multi(&self, queries: Vec<String>) -> DBResultReceiver;
    fn new_query<T: AsRef<str>>(&self, query: T) -> DBResultReceiver {
        self.new_query_multi(vec![query.as_ref().to_string()])
    }
}

impl NewQuery for DBQuerySender {
    fn new_query_multi(&self, queries: Vec<String>) -> DBResultReceiver {
        let (result_tx, result_rx) = mpsc::channel(1);
        let query_task = self
            .clone()
            .send((queries, result_tx))
            .map(|_| ())
            .map_err(|_| ());
        tokio::spawn(query_task);
        result_rx
    }
}
// }

// pub fn new_query<F>(query: String, query_tx: DBQuerySender) -> DBResultReceiver {
//     let (result_tx,result_rx) = mpsc::channel(1);
//     tokio::spawn(query_tx.send((query, result_tx)).map(|_|()).map_err(|_|())); //TODO future result
//     result_rx
// }

// {
//     let peers_tx2 = peers_tx.clone();
//     let db_result_task = result_rx.take(1).for_each(move|result|{
//         println!("result {:?}", result);
//         peers_tx2.lock_for_each(|(id,tx)|{
//             tx.start_send(command::S2C::Result_Login(result.get(0).unwrap().get(0).unwrap().clone()));
//             tx.poll_complete();
//         });
//         Ok(())
//     });
//     tasks.push(Box::new(db_result_task));
// }
