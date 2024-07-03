use crate::error::Error;
use ics23::InnerOp;
use secp256k1::rand::{self, Rng};
use std::cmp::PartialEq;
use types::ibc_core::ics04_channel::packet::Packet;

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

fn compute_overlap_matrix(paths: &Vec<Vec<InnerOp>>) -> Vec<Vec<usize>> {
    let n = paths.len();
    let mut overlap = vec![vec![0; n]; n];

    for i in 0..n {
        for j in i..n {
            let count = compute_overlap(&paths[i], &paths[j]);
            overlap[i][j] = count;
            overlap[j][i] = count;
        }
    }
    overlap
}
#[derive(Clone, Debug)]
struct Cluster {
    center: usize,
    packets: Vec<Packet>,
}

impl Cluster {
    fn new(packets: &Vec<Packet>, num: usize) -> Vec<Cluster> {
        let packet_len = packets.len();

        let mut rng = rand::thread_rng();
        let mut selected_numbers = Vec::new();

        for i in 0..num{
            let number = rng.gen_range(0..packet_len);
            selected_numbers.push(number);
        }
        let mut clusters:Vec<Cluster>=Vec::new();
        for i in 0..num{
            let c = Cluster{
                center:selected_numbers[i],
                packets:{ let mut p = Vec::new();
                    
                    p.push(packets[selected_numbers[i]].clone());
                    p
                },
            };
            clusters.push(c);
        }
        clusters
    }
    fn group(clusters:&mut Vec<Cluster>,matrix:&Vec<Vec<usize>>,packets: &Vec<Packet>){
        for i in 0..packets.len(){
            let mut min_distance:usize = 0;
            let mut closest_cluster = 0;
            for (j,cluster)in clusters.iter().enumerate(){
                let mut distance:usize = 0;
                if i<cluster.center{
                    distance = matrix[i][cluster.center];
                }else{
                    distance = matrix[cluster.center][i];
                }
                if distance >= min_distance{
                    min_distance = distance;
                    closest_cluster = j
                }
            }
            clusters[closest_cluster].packets.push(packets[i].clone());
        }
    }
}
