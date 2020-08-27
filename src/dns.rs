use crate::Result;

use trust_dns_server as dns;
use dns::proto;
use proto::serialize::binary::{BinDecoder, BinDecodable, BinEncoder};
use proto::op::header::MessageType;
use proto::op::{OpCode, ResponseCode, Header};
use proto::rr::{DNSClass, Record, RecordType};
use proto::rr::domain::{Name, Label};
use dns::authority::{MessageRequest, MessageResponse, MessageResponseBuilder};

use std::collections::BTreeMap;
use std::sync::Arc;
use std::net::Ipv4Addr;


/// time-to-live [s]
static TTL: u32 = 5 * 60;

pub struct DnsAuthority {
    names: Arc<BTreeMap<String, (Name, Ipv4Addr)>>,
}

impl DnsAuthority {
    pub fn new(names: Vec<(String, Ipv4Addr)>) -> Result<DnsAuthority> {
        let mut map = BTreeMap::new();
        for (domain, addr) in names {
            let name = to_name(domain.as_str())?;
            map.insert(domain, (name, addr));
        }
        info!("zone contents:\n{:?}", map.iter().collect::<Vec<(&String, &(Name, Ipv4Addr))>>());

        Ok(DnsAuthority {
            names: Arc::new(map),
        })
    }

    pub fn answer_query(&self, raw_content: Vec<u8>) -> Result<Vec<u8>> {
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

        let answers: Vec<Record> = request.queries().iter()
            .filter(|q| match (q.query_type(), q.query_class()) {
                (RecordType::A, DNSClass::IN) => true,
                (RecordType::AAAA, DNSClass::IN) => true,
                (t, c) => {
                    trace!("received unsupported query: ({}, {})", t, c);
                    false
                }
            })
            .map(|q| {
                let q_name = q.name().to_string();
                let q_name = &q_name.as_str()[..q_name.len() - 1];  // cut off trailing "."
                trace!("query name: {}", q_name);

                self.names.get(q_name)
                    .map(|(n, addr)| {
                        let mut r = Record::with(n.to_owned(), RecordType::A, TTL);
                        r.set_rdata(proto::rr::RData::A(addr.to_owned()));
                        r
                    })
            })
            .filter(Option::is_some)
            .map(|o| o.unwrap())
            .collect();

        if answers.is_empty() {
            let response_bytes = respond_with(&request, ResponseCode::NXDomain)?;
            return Ok(response_bytes);
        }

        // finalize and send message
        let header = {
            let mut header = Header::new();
            header.set_id(request.id())
                .set_op_code(request.op_code())
                .set_response_code(ResponseCode::NoError)
                .set_authoritative(true)
                .set_checking_disabled(false)
                .set_message_type(MessageType::Response)
                .set_answer_count(answers.len() as u16);
            header
        };
        let answers: Box<dyn Iterator<Item = &Record> + Send> = Box::new(answers.iter());
        let response = MessageResponseBuilder::new(None)
            .build(header, answers, none(), none(), none());

        let response_bytes = encode(response)?;
        Ok(response_bytes)
    }
}

fn respond_with(request: &MessageRequest, response_code: proto::op::ResponseCode) -> Result<Vec<u8>> {
    let builder = MessageResponseBuilder::new(None);
    let response = builder.error_msg(request.id(), request.op_code(), response_code);
    encode(response)
}

fn encode(response: MessageResponse) -> Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(512);
    let mut encoder = BinEncoder::new(&mut buf);
    response.destructive_emit(&mut encoder)?;
    Ok(buf)   
}

fn none() -> Box<dyn Iterator<Item = &'static Record> + Send> {
    Box::new(std::iter::empty())
}

fn to_name(domain: &str) -> Result<Name> {
    let labels: std::result::Result<Vec<Label>, _> = domain.split(".")
        .collect::<Vec<&str>>()
        .into_iter()
        .map(|l| Label::from_utf8(l))
        .collect();
    let labels = labels?;
    let name = Name::from_labels(labels)?;
    Ok(name)
}