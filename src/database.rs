use std::{
    sync::{
        Mutex,
    },
    thread,
};
use futures::{
    prelude::*,
    sync::mpsc,
};
// use diesel::prelude::*;
// use diesel::MysqlConnection;
// use diesel::connection::SimpleConnection;
use dotenv::dotenv;
use mysql;

pub type DBResult = mysql::Row;
pub type DBError = ();
pub type DBQuery = String;

pub type DBResultSender = futures::sync::mpsc::Sender<DBResult>;
pub type DBResultReceiver  = futures::sync::mpsc::Receiver<DBResult>;
pub type DBQuerySender = futures::sync::mpsc::Sender<(DBQuery,DBResultSender)>;

    // mysql.batch_execute("INSERT INTO users(id,name) VALUES(1,'test')").expect("exec err");

// impl mysql_common::value::convert::FromValue for Vec<String> {

// }

pub fn start_database() -> (impl Future<Item=(),Error=()>,DBQuerySender) {
    let mysql = mysql::Pool::new_manual(1, 1, "mysql://rustgs:@localhost/rustgs").expect("db conn err");
    println!("db connected");
    let (query_tx,query_rx) = mpsc::channel::<(DBQuery,mpsc::Sender<DBResult>)>(24);

    let task = query_rx.for_each(move|(query,result_tx)|{

        for row in mysql.prep_exec(query,()).unwrap() {
            // let value:String = mysql::from_row(row.unwrap());
            // println!("selected value:{}", value);
            // let result = vec![value];
            let row = row.unwrap();
            let send_task = result_tx.clone().send(row).map(|_|()).map_err(|_|());
            tokio::spawn(send_task);
        }

        Ok(())
    });
    (task,query_tx)
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
    fn new_query<T:AsRef<str>>(&self, query: T) -> DBResultReceiver;
}

impl NewQuery for DBQuerySender {
    fn new_query<T:AsRef<str>>(&self, query: T) -> DBResultReceiver {
        let (result_tx,result_rx) = mpsc::channel(1);
        let query_task = self.clone().send((query.as_ref().to_string(), result_tx)).map(|_|()).map_err(|_|());
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