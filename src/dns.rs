
// use dns_message_parser as dns;
use trust_dns_server as dns;
use dns::proto;
use proto::serialize::binary::{BinDecoder, BinDecodable, BinEncoder};
use proto::op::header::MessageType;
use proto::op::{OpCode, ResponseCode};
// use proto::op::query::Query;
// use proto::rr::{DNSClass, RecordType};
use dns::authority::{MessageRequest, MessageResponseBuilder};

pub fn answer_query(raw_content: Vec<u8>) -> std::result::Result<Vec<u8>, anyhow::Error> {
    let mut decoder = BinDecoder::new(&raw_content);
    let request = MessageRequest::read(&mut decoder)
        .map_err(|e| anyhow!("dns decode error: {:?}", e))?;
    trace!("received request: \n{:#?}", request);

    if request.message_type() != MessageType::Query {
        trace!("received none-query, returning NotImplemented response.");
        let response_bytes = respond_with(&request, ResponseCode::NotImp)?;
        return Ok(response_bytes);
    }
    if request.op_code() != OpCode::Query {
        trace!("received none-standard query, ignoring.");
        let response_bytes = respond_with(&request, ResponseCode::NotImp)?;
        return Ok(response_bytes);
    }

    // let flags = dns::Flags {
    //     qr: true,
    //     opcode: dns::Opcode::Query,
    //     // authorative answer
    //     aa: true,
    //     // truncated
    //     tc: false,
    //     // recursion desired
    //     rd: false,
    //     // recursion available
    //     ra: false,
    //     // dnssec
    //     ad: false,
    //     cd: false,
    //     rcode: dns::RCode::NoError,
    // };
    // let mut builder = dns::Dns::new(1337, flags, vec![], )

    // let mut builder = MessageResponseBuilder::new(None);
    //     builder.
    // for q in packet.queries() {
    //     if q.query_type() != RecordType::A {
    //         trace!("received none-A query, ignoring.");
    //         continue;
    //     }
        
    // }
    let response_bytes = respond_with(&request, ResponseCode::NotAuth)?;
    Ok(response_bytes)
}

fn respond_with(request: &MessageRequest, response_code: proto::op::ResponseCode) -> Result<Vec<u8>, anyhow::Error> {
    let builder = MessageResponseBuilder::new(None);
    let response = builder.error_msg(request.id(), request.op_code(), response_code);
    
    let mut buf = Vec::with_capacity(512);
    let mut encoder = BinEncoder::new(&mut buf);
    response.destructive_emit(&mut encoder)?;
    Ok(buf)
}