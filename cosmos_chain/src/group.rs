use crate::{channel::Channel, channel_pool::ChannelPool, error::Error, event_pool::CTXGroup};
use ics23::InnerOp;
use secp256k1::rand::{self, Rng};
use std::{cmp::PartialEq, collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use types::{
    ibc_core::{ics02_client::height::Height, ics04_channel::packet::Packet},
    ibc_events::{IbcEvent, IbcEventWithHeight},
};

struct ComparableInnerOp {
    inner_op: InnerOp,
}

impl From<InnerOp> for ComparableInnerOp {
    fn from(inner_op: InnerOp) -> Self {
        ComparableInnerOp { inner_op }
    }
}

impl PartialEq for ComparableInnerOp {
    fn eq(&self, other: &Self) -> bool {
        self.inner_op.hash == other.inner_op.hash
            && self.inner_op.prefix == other.inner_op.prefix
            && self.inner_op.suffix == other.inner_op.suffix
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl Eq for ComparableInnerOp {}

fn compute_overlap(path1: &Vec<InnerOp>, path2: &Vec<InnerOp>) -> usize {
    let len1 = path1.len();
    let len2 = path2.len();
    let min_len = std::cmp::min(len1, len2);
    let mut count = 8;
    for i in 0..min_len {
        if path1[min_len - 1 - i] == path2[min_len - 1 - i] {
            count += 1;
        } else {
            break;
        }
    }
    count
}

pub fn compute_overlap_matrix(paths: &Vec<Vec<InnerOp>>) -> Vec<Vec<usize>> {
    let n = paths.len();
    let mut overlap = vec![vec![0; n]; n];

    for i in 0..n {
        for j in (i + 1)..n {
            let count = compute_overlap(&paths[i], &paths[j]);
            overlap[i][j] = count;
            overlap[j][i] = count;
        }
    }
    overlap
}
#[derive(Clone, Debug)]
pub struct CTX {
    num: usize,
    packet: Packet,
    distance: usize,
}

impl CTX {
    pub fn sort_by_distance(ctxs: &mut Vec<CTX>) {
        ctxs.sort_by(|a, b| a.distance.cmp(&b.distance));
    }
}

#[derive(Clone, Debug)]
pub struct Cluster {
    center: usize,
    ctxs: Vec<CTX>,
}

impl Cluster {
    pub fn new(packets: &Vec<Packet>, num: usize) -> (Vec<Cluster>, Vec<CTX>) {
        let packet_len = packets.len();

        let mut rng = rand::thread_rng();
        let mut selected_numbers = Vec::new();
        let mut ctxs: Vec<CTX> = vec![];
        for i in 0..packet_len {
            let ctx = CTX {
                num: i,
                packet: packets[i].clone(),
                distance: 0,
            };
            ctxs.push(ctx);
        }

        for i in 0..num {
            let number = rng.gen_range(0..packet_len);
            selected_numbers.push(number);
        }
        let mut clusters: Vec<Cluster> = Vec::new();
        for i in 0..num {
            let c = Cluster {
                center: selected_numbers[i],
                ctxs: {
                    let mut p: Vec<CTX> = Vec::new();

                    p.push(ctxs[selected_numbers[i]].clone());
                    p
                },
            };
            clusters.push(c);
        }
        (clusters, ctxs)
    }
    pub fn get_ctxs(&self) -> &Vec<CTX> {
        &self.ctxs
    }
    pub fn group(clusters: &mut Vec<Cluster>, matrix: &Vec<Vec<usize>>, ctxs: &mut Vec<CTX>) {
        for i in 0..ctxs.len() {
            let mut min_distance: usize = 0;
            let mut closest_cluster = 0;
            for (j, cluster) in clusters.iter().enumerate() {
                let mut distance: usize = 0;
                if ctxs[i].num < cluster.center {
                    distance = matrix[ctxs[i].num][cluster.center];
                } else {
                    distance = matrix[cluster.center][ctxs[i].num];
                }
                if distance >= min_distance {
                    min_distance = distance;
                    closest_cluster = j;
                    ctxs[i].distance = min_distance;
                }
            }
            if i == clusters[closest_cluster].center{
                continue;
            }
            println!("closest_cluster:{}", closest_cluster.clone());
            clusters[closest_cluster].ctxs.push(ctxs[i].clone());
        }
    }
}

pub fn adjust_group(clusters: &mut Vec<Cluster>, matrix: &Vec<Vec<usize>>,groupsize:usize) -> Vec<Cluster> {
    let mut old_clusters: Vec<Cluster> = clusters.clone();
    let mut result_clusters:Vec<Cluster>=vec![];
    loop {
        
        if old_clusters.len() == 1 {
            result_clusters.push(old_clusters[0].clone());
            break;
        }
        if old_clusters.len() == 0 {
            
            break;
        }
        println!("现在的cluster个数是：{}",old_clusters.len());
        let mut extra_ctxs = Vec::new();
        let mut flag = 0;
        for cluster in old_clusters.clone(){
            if cluster.ctxs.len()>groupsize{
                 flag = 1;
                 break;
            }
        };
        if flag == 0{
            for cluster in old_clusters.clone(){
                result_clusters.push(cluster.clone());
            }
            break;
        };
        old_clusters.retain(|cluster| {
            if cluster.ctxs.len() > groupsize {
                // 创建一个可变副本来处理排序和分割
                let mut cluster_clone = cluster.clone();
                CTX::sort_by_distance(&mut cluster_clone.ctxs);
    
                // 将大于 groupsize 的 CTX 放入 extra_ctxs 中
                let excess = cluster_clone.ctxs.split_off(groupsize);
                println!("将超过的交易去除后现在的groupsize是：{}",cluster_clone.ctxs.len());
                extra_ctxs.extend(excess);
                result_clusters.push(cluster_clone.clone());
                // 返回 false 以从 old_clusters 中删除当前 Cluster
                false
            } else {
                // 返回 true 以保留当前 Cluster
                true
            }
        });
        Cluster::group(&mut old_clusters, matrix, &mut extra_ctxs);
    }
    return result_clusters
}

pub async fn make_groups(
    events: &mut Vec<IbcEventWithHeight>,
    height: &Height,
    channelkey: &String,
    channelop: Option<Channel>,
    groupsize:usize
) -> Vec<CTXGroup> {
    let mut packets: Vec<Packet> = vec![];
    let mut records: HashMap<Packet, IbcEventWithHeight> = HashMap::new();
    let mut paths: Vec<Vec<InnerOp>> = vec![];
    let mut groups: Vec<CTXGroup> = vec![];
    for e in events.clone() {
        let p = match e.event.clone() {
            IbcEvent::SendPacket(evt) => evt.packet,
            _ => continue,
        };
        packets.push(p.clone());
        records.insert(p.clone(), e.clone());
    }

    // let channelop = cp.read().await.query_channel_by_key(&channelkey);
    if let Some(channel) = channelop {
        let packet_proof_map = channel
            .source_chain()
            .query_packets_merkle_proof_infos(packets.clone(), height)
            .await;
        if let Ok(hashmap) = packet_proof_map {
            for p in packets.clone() {
                if let Some(proof_info) = hashmap.get(&p) {
                    paths.push(proof_info.full_path.clone());
                }
            }
        }
        let matrix = compute_overlap_matrix(&paths);
        println!("matrix!!!!!");
        for row in matrix.clone() {
            for element in row {
                print!("{} ", element);
            }
            println!();
        }
        let num = (events.len() + groupsize - 1) / groupsize;
        println!("一共有{}笔交易需要分组",events.len());
        println!("需要分成{}组",num);
        // let num = 5;
        // let clone_packets = packets.clone();
        let (mut clusters, mut ctxs) = Cluster::new(&packets.clone(), num);
        Cluster::group(&mut clusters, &matrix.clone(), &mut ctxs);
        let result_clusters = adjust_group(&mut clusters,&matrix.clone(),groupsize);
        for cs in result_clusters {
            let mut ctxgroup: CTXGroup = vec![];
            let ps = cs.get_ctxs();
            for p in ps {
                if let Some(record) = records.get(&p.packet) {
                    ctxgroup.push(record.clone());
                }
            }
            groups.push(ctxgroup);
        }
    }
    return groups;
}
