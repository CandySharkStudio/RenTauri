use aes::cipher::{BlockDecryptMut, KeyInit};
use aes::Aes256;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
const BUFFER_SIZE: usize = 16 * 1024;
const BLOCK_SIZE: usize = 16;
/// 用于记录当前正在往哪个 Vec 里写，写了多少
struct WriteTask {
    path: String,
    total_size: u64,
    written: u64,
}
/// 流式解密直接进入内存 HashMap
pub fn decrypt_to_memory(
    input_path: String,
    key: [u8; 32],
) -> Result<(HashMap<String, Vec<u8>>, Option<String>), Box<dyn std::error::Error>> {
    let mut in_file = File::open(input_path)?;
    // 先提取目录文件大小！
    let mut dir_size_buf = [0u8; 4];
    in_file.read_exact(&mut dir_size_buf)?;
    let dir_encrypted_size = u32::from_le_bytes(dir_size_buf) as usize;
    // 再提取目录文件
    let mut dir_ciphertext = vec![0u8; dir_encrypted_size];
    in_file.read_exact(&mut dir_ciphertext)?;
    // 提取 iv1
    let mut iv1 = [0u8; 16];
    iv1.copy_from_slice(&dir_ciphertext[0..16]);
    let dir_ct = &dir_ciphertext[16..];
    // 解密目录（密钥错了这里不会报错，但是下面填充时会报错！）
    let mut cipher1 = Aes256::new_from_slice(&key).map_err(|e| e.to_string())?;
    let mut dir_plaintext = vec![0u8; dir_ct.len()];
    let mut prev1 = iv1;
    for (i, chunk) in dir_ct.chunks_exact(16).enumerate() {
        let mut curr_ct = [0u8; 16];
        curr_ct.copy_from_slice(chunk);
        let mut block = curr_ct;
        cipher1.decrypt_block_mut((&mut block).into());
        for j in 0..16 {
            block[j] ^= prev1[j];
        }
        dir_plaintext[i * 16..(i + 1) * 16].copy_from_slice(&block);
        prev1 = curr_ct;
    }
    // 如果密钥错误，那么这里直接抛出报错，在通过 iv1 解密目录时就已经失败了。。
    let pad_val = dir_plaintext.last().ok_or("目录数据为空")?;
    if *pad_val == 0
        || *pad_val > 16
        || dir_plaintext[dir_plaintext.len() - (*pad_val as usize)..]
            .iter()
            .any(|&b| b != *pad_val)
    {
        return Err("密钥错误或文件损坏".into());
    }
    let raw_dir = &dir_plaintext[..dir_plaintext.len() - (*pad_val as usize)];
    // 解密目录
    let mut offset = 0;
    let file_count = u32::from_le_bytes(raw_dir[offset..offset + 4].try_into()?) as usize;
    offset += 4;
    let mut result_map: HashMap<String, Vec<u8>> = HashMap::with_capacity(file_count);
    let mut write_queue: Vec<WriteTask> = Vec::with_capacity(file_count);
    let mut total_valid_size: u64 = 0;
    // 记录真实的 main.lua 的文件名！以便于后面直接读取。
    let mut last_file_name: Option<String> = None;
    for _ in 0..file_count {
        let path_len = u32::from_le_bytes(raw_dir[offset..offset + 4].try_into()?) as usize;
        offset += 4;
        let path_string = String::from_utf8(raw_dir[offset..offset + path_len].to_vec())?;
        offset += path_len;
        let file_size = u64::from_le_bytes(raw_dir[offset..offset + 8].try_into()?);
        offset += 8;
        total_valid_size += file_size;
        result_map.insert(path_string.clone(), Vec::with_capacity(file_size as usize));
        write_queue.push(WriteTask {
            path: path_string.clone(),
            total_size: file_size,
            written: 0,
        });
        last_file_name = Some(path_string);
    }
    // 读取 iv2
    let mut iv2 = [0u8; 16];
    in_file.read_exact(&mut iv2)?;
    // 开始解密明文目录！
    let mut cipher2 = Aes256::new_from_slice(&key).map_err(|e| e.to_string())?;
    let mut prev_block = iv2;
    let mut remaining_bytes = total_valid_size;
    let mut current_idx = 0;
    let mut ct_buf = vec![0u8; BUFFER_SIZE];
    let mut pt_buf = vec![0u8; BUFFER_SIZE];
    loop {
        let bytes_read = in_file.read(&mut ct_buf)?;
        if bytes_read == 0 {
            break;
        }
        let mut pt_idx = 0;
        // 逐块解密
        for chunk in ct_buf[..bytes_read].chunks_exact(BLOCK_SIZE) {
            let mut curr_ct = [0u8; BLOCK_SIZE];
            curr_ct.copy_from_slice(chunk);
            let mut block = curr_ct;
            cipher2.decrypt_block_mut((&mut block).into());
            for i in 0..BLOCK_SIZE {
                block[i] ^= prev_block[i];
            }
            prev_block = curr_ct;
            pt_buf[pt_idx..pt_idx + BLOCK_SIZE].copy_from_slice(&block);
            pt_idx += BLOCK_SIZE;
        }
        // 将解密出的数据分发到 HashMap 中对应的 Vec
        let mut buf_offset = 0;
        while buf_offset < pt_idx {
            if current_idx >= write_queue.len() {
                break; // 所有坑都填满了，剩下的是 PKCS7 填充，直接丢弃
            }
            let task = &mut write_queue[current_idx];
            let remaining_in_file = task.total_size - task.written;
            let remaining_in_buf = pt_idx - buf_offset;
            let bytes_to_write = std::cmp::min(remaining_in_file as usize, remaining_in_buf);
            if bytes_to_write == 0 {
                // 当前文件填满，切换到下一个
                current_idx += 1;
                continue;
            }
            // 获取 HashMap 中对应的 Vec 并写入
            if let Some(buffer) = result_map.get_mut(&task.path) {
                buffer.extend_from_slice(&pt_buf[buf_offset..buf_offset + bytes_to_write]);
            }
            buf_offset += bytes_to_write;
            task.written += bytes_to_write as u64;
            remaining_bytes -= bytes_to_write as u64;
        }
        if remaining_bytes == 0 {
            break; // 提前结束，不读最后无用的填充块
        }
    }
    Ok((result_map, last_file_name))
}
