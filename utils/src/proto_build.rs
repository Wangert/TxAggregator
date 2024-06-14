use std::io::Result;

pub fn proto_build_read() -> Result<()> {
    // tonic_build::compile_protos("C:/Users/admin/Documents/GitHub/TxAggregator/utils/src/protos/hello.proto");
    tonic_build::configure()
        .build_server(true) // 是否编译生成用于服务端的代码
        .build_client(true) // 是否编译生成用于客户端的代码
        .out_dir("C:/Users/admin/Documents/GitHub/TxAggregator/utils/src/proto")  // 输出的路径，此处指定为项目根目录下的protos目录
        // 指定要编译的proto文件路径列表，第二个参数是提供protobuf的扩展路径，
        // 因为protobuf官方提供了一些扩展功能，自己也可能会写一些扩展功能，
        // 如存在，则指定扩展文件路径，如果没有，则指定为proto文件所在目录即可
        .compile(&["C:/Users/admin/Documents/GitHub/TxAggregator/utils/src/proto/hello.proto"], &["proto"])?; 
    Ok(())
}
#[cfg(test)]
pub mod proto_build_test{
    use super::proto_build_read;


    #[test]
    pub fn build_work(){
        println!("start");
        proto_build_read();
        println!("over");
    }
}